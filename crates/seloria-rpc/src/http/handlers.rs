use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::body::Body;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use async_compression::tokio::bufread::GzipEncoder;
use seloria_consensus::{
    verify_qc, BlockBuilder, BlockBuilderConfig, CommitRequest, CommitResponse, ProposeRequest,
    ProposeResponse, Validator, ValidatorEndpoint,
};
use seloria_core::{Account, Block, Claim, Hash, KeyPair, KvValue, PublicKey, Transaction};
use seloria_mempool::Mempool;
use seloria_state::{ChainState, Storage};
use seloria_vm::validate_transaction;
use serde::{Deserialize, Serialize};
use tokio::io::BufReader;
use tokio::sync::RwLock;
use tokio_util::io::ReaderStream;
use tracing::info;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::RpcError;
use crate::ws::events::EventBroadcaster;

/// Application state shared with handlers
pub struct AppState<S: Storage> {
    pub chain_state: Arc<RwLock<ChainState<S>>>,
    pub mempool: Arc<Mempool>,
    pub broadcaster: Arc<EventBroadcaster>,
    pub validator_keypair: Option<Arc<tokio::sync::Mutex<KeyPair>>>,
    pub issuer_keypair: Option<Arc<tokio::sync::Mutex<KeyPair>>>,
    pub snapshot_path: Option<PathBuf>,
    pub validator_endpoints: Vec<ValidatorEndpoint>,
    pub faucet_keypair: Option<Arc<tokio::sync::Mutex<KeyPair>>>,
    pub faucet_lock: Arc<tokio::sync::Mutex<()>>,
}

// Response types

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub chain_id: u64,
    pub height: u64,
    pub head_block_hash: Option<String>,
    pub mempool_size: usize,
}

#[derive(Debug, Serialize)]
pub struct TxSubmitResponse {
    pub hash: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct AccountResponse {
    pub pubkey: String,
    pub balance: u64,
    pub nonce: u64,
    pub total_balance: u64,
}

#[derive(Debug, Serialize)]
pub struct BlockResponse {
    pub height: u64,
    pub hash: String,
    pub prev_hash: String,
    pub timestamp: u64,
    pub tx_count: usize,
    pub proposer: String,
    pub tx_root: String,
    pub state_root: String,
}

#[derive(Debug, Serialize)]
pub struct ClaimResponse {
    pub id: String,
    pub claim_type: String,
    pub payload_hash: String,
    pub creator: String,
    pub creator_stake: u64,
    pub yes_stake: u64,
    pub no_stake: u64,
    pub status: String,
    pub created_at: u64,
    pub attestation_count: usize,
}

#[derive(Debug, Serialize)]
pub struct KvResponse {
    pub ns_id: String,
    pub key: String,
    pub codec: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct KvKeysResponse {
    pub ns_id: String,
    pub keys: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SnapshotMetaResponse {
    pub chain_id: u64,
    pub height: u64,
    pub head_block_hash: Option<String>,
    pub snapshot_size: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct SnapshotPublishResponse {
    pub download_url: String,
}

// Request types

#[derive(Debug, Deserialize)]
pub struct TxSubmitRequest {
    pub transaction: Transaction,
}

#[derive(Debug, Deserialize)]
pub struct FaucetRequest {
    pub to_pubkey: String,
    pub amount: u64,
}

#[derive(Debug, Serialize)]
pub struct FaucetResponse {
    pub to_pubkey: String,
    pub amount: u64,
    pub tx_hash: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct IssueCertRequest {
    pub agent_pubkey: String,
    pub issued_at: u64,
    pub expires_at: u64,
    pub capabilities: Vec<seloria_core::Capability>,
    pub metadata_hash: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IssueCertResponse {
    pub cert: seloria_core::SignedAgentCertificate,
}

// Handlers

/// GET /status - Get node status
pub async fn get_status<S: Storage + Send + Sync>(
    State(state): State<Arc<AppState<S>>>,
) -> Result<Json<StatusResponse>, RpcError> {
    let chain_state = state.chain_state.read().await;
    let mempool_size = state.mempool.size().await;

    let head_block_hash = if let Some(ref block) = chain_state.head_block {
        Some(block.hash().map(|h| h.to_hex()).unwrap_or_default())
    } else {
        None
    };

    Ok(Json(StatusResponse {
        chain_id: chain_state.chain_id,
        height: chain_state.height,
        head_block_hash,
        mempool_size,
    }))
}

/// GET /snapshot/meta - Get snapshot metadata
pub async fn get_snapshot_meta<S: Storage + Send + Sync>(
    State(state): State<Arc<AppState<S>>>,
) -> Result<Json<SnapshotMetaResponse>, RpcError> {
    let chain_state = state.chain_state.read().await;

    let head_block_hash = if let Some(ref block) = chain_state.head_block {
        Some(block.hash().map(|h| h.to_hex()).unwrap_or_default())
    } else {
        None
    };

    let snapshot_size = if let Some(path) = &state.snapshot_path {
        match tokio::fs::metadata(path).await {
            Ok(meta) => Some(meta.len()),
            Err(_) => None,
        }
    } else {
        None
    };

    Ok(Json(SnapshotMetaResponse {
        chain_id: chain_state.chain_id,
        height: chain_state.height,
        head_block_hash,
        snapshot_size,
    }))
}

/// GET /snapshot - Download latest snapshot file
pub async fn get_snapshot<S: Storage + Send + Sync>(
    State(state): State<Arc<AppState<S>>>,
    headers: HeaderMap,
) -> Result<Response, RpcError> {
    let path = state.snapshot_path.as_ref().ok_or_else(|| {
        RpcError::NotFound("Snapshot not available on this node".to_string())
    })?;

    let file = tokio::fs::File::open(path)
        .await
        .map_err(|_| RpcError::NotFound("Snapshot file not found".to_string()))?;

    let accept_encoding = headers
        .get(header::ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let (body, content_encoding) = if accept_encoding.contains("gzip") {
        let reader = BufReader::new(file);
        let encoder = GzipEncoder::new(reader);
        let stream = ReaderStream::new(encoder);
        (Body::from_stream(stream), Some("gzip"))
    } else {
        let stream = ReaderStream::new(file);
        (Body::from_stream(stream), None)
    };

    let mut response = Response::new(body);
    *response.status_mut() = StatusCode::OK;
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, header::HeaderValue::from_static("application/octet-stream"));
    if let Some(encoding) = content_encoding {
        response.headers_mut().insert(
            header::CONTENT_ENCODING,
            header::HeaderValue::from_static(encoding),
        );
    }
    Ok(response)
}

/// POST /snapshot/publish - Upload snapshot to a presigned URL and return download URL
pub async fn publish_snapshot<S: Storage + Send + Sync>(
    State(state): State<Arc<AppState<S>>>,
) -> Result<Json<SnapshotPublishResponse>, RpcError> {
    let upload_url = std::env::var("SNAPSHOT_UPLOAD_URL")
        .map_err(|_| RpcError::BadRequest("SNAPSHOT_UPLOAD_URL not set".to_string()))?;
    let download_url = std::env::var("SNAPSHOT_DOWNLOAD_URL")
        .map_err(|_| RpcError::BadRequest("SNAPSHOT_DOWNLOAD_URL not set".to_string()))?;

    let path = state.snapshot_path.as_ref().ok_or_else(|| {
        RpcError::NotFound("Snapshot not available on this node".to_string())
    })?;

    let file = tokio::fs::File::open(path)
        .await
        .map_err(|_| RpcError::NotFound("Snapshot file not found".to_string()))?;
    let stream = ReaderStream::new(file);

    let client = reqwest::Client::new();
    let response = client
        .put(upload_url)
        .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
        .body(reqwest::Body::wrap_stream(stream))
        .send()
        .await
        .map_err(|e| RpcError::Internal(format!("Snapshot upload failed: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(RpcError::Internal(format!(
            "Snapshot upload failed: {} {}",
            status, body
        )));
    }

    Ok(Json(SnapshotPublishResponse { download_url }))
}

/// POST /tx - Submit a transaction
pub async fn submit_tx<S: Storage + Send + Sync>(
    State(state): State<Arc<AppState<S>>>,
    headers: HeaderMap,
    Json(request): Json<TxSubmitRequest>,
) -> Result<Json<TxSubmitResponse>, RpcError> {
    let tx = request.transaction;

    // Verify signature first
    tx.verify_signature()?;

    // Basic validation against current state (prevents invalid mempool spam)
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let chain_state = state.chain_state.read().await;
    let validation = validate_transaction(&tx, &chain_state, current_time);
    if !validation.is_valid {
        let msg = validation
            .error
            .map(|e| e.to_string())
            .unwrap_or_else(|| "Invalid transaction".to_string());
        return Err(RpcError::BadRequest(msg));
    }
    drop(chain_state);

    let hash = tx.hash()?;
    let hash_hex = hash.to_hex();

    let is_gossip = headers
        .get("x-gossip")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == "1")
        .unwrap_or(false);

    // Add to mempool (ignore duplicate gossip)
    match state.mempool.add(tx.clone()).await {
        Ok(_) => {}
        Err(seloria_mempool::MempoolError::AlreadyExists) => {
            if !is_gossip {
                return Err(RpcError::Mempool(seloria_mempool::MempoolError::AlreadyExists));
            }
        }
        Err(e) => return Err(RpcError::Mempool(e)),
    }

    info!("Transaction {} submitted to mempool", hash_hex);

    if !is_gossip && !state.validator_endpoints.is_empty() {
        let endpoints = state.validator_endpoints.clone();
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let client = reqwest::Client::new();
            for endpoint in endpoints {
                let url = format!("{}/tx", endpoint.address.trim_end_matches('/'));
                let _ = client
                    .post(url)
                    .header("x-gossip", "1")
                    .json(&serde_json::json!({ "transaction": tx_clone }))
                    .send()
                    .await;
            }
        });
    }

    Ok(Json(TxSubmitResponse {
        hash: hash_hex,
        status: "pending".to_string(),
    }))
}

/// POST /faucet - Mint testnet funds via a faucet transfer tx
pub async fn faucet<S: Storage + Send + Sync>(
    State(state): State<Arc<AppState<S>>>,
    headers: HeaderMap,
    Json(request): Json<FaucetRequest>,
) -> Result<Json<FaucetResponse>, RpcError> {
    let faucet_key = std::env::var("FAUCET_KEY").ok();
    if let Some(expected) = faucet_key {
        let provided = headers
            .get("x-faucet-key")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if provided != expected {
            return Err(RpcError::BadRequest("Invalid faucet key".to_string()));
        }
    }

    let max_amount: u64 = std::env::var("FAUCET_MAX")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1_000_000);

    if request.amount == 0 || request.amount > max_amount {
        return Err(RpcError::BadRequest(format!(
            "Invalid amount (max {})",
            max_amount
        )));
    }

    let faucet_keypair = state
        .faucet_keypair
        .as_ref()
        .ok_or_else(|| RpcError::BadRequest("Faucet not configured".to_string()))?
        .lock()
        .await
        .clone();

    let _guard = state.faucet_lock.lock().await;

    let to_pubkey = PublicKey::from_hex(&request.to_pubkey)
        .map_err(|_| RpcError::BadRequest("Invalid recipient pubkey".to_string()))?;

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let nonce = {
        let chain_state = state.chain_state.read().await;
        if !chain_state.is_certified_agent(&faucet_keypair.public, current_time) {
            return Err(RpcError::BadRequest(
                "Faucet is not registered as a certified agent".to_string(),
            ));
        }
        chain_state
            .get_account(&faucet_keypair.public)
            .map(|a| a.nonce + 1)
            .unwrap_or(1)
    };

    let mut tx = Transaction::new(
        faucet_keypair.public,
        nonce,
        0,
        vec![seloria_core::Op::Transfer {
            to: to_pubkey,
            amount: request.amount,
        }],
    );
    tx.sign(&faucet_keypair.secret)?;

    let chain_state = state.chain_state.read().await;
    let validation = validate_transaction(&tx, &chain_state, current_time);
    if !validation.is_valid {
        let msg = validation
            .error
            .map(|e| e.to_string())
            .unwrap_or_else(|| "Invalid faucet transaction".to_string());
        return Err(RpcError::BadRequest(msg));
    }
    drop(chain_state);

    let hash = tx.hash()?;
    state.mempool.add(tx).await?;

    Ok(Json(FaucetResponse {
        to_pubkey: request.to_pubkey,
        amount: request.amount,
        tx_hash: hash.to_hex(),
        status: "pending".to_string(),
    }))
}

/// GET /tx/:hash - Get transaction by hash
pub async fn get_tx<S: Storage + Send + Sync>(
    State(state): State<Arc<AppState<S>>>,
    Path(hash_hex): Path<String>,
) -> Result<Json<Transaction>, RpcError> {
    let hash = Hash::from_hex(&hash_hex).map_err(|_| RpcError::BadRequest("Invalid hash".to_string()))?;

    // Check mempool first
    if let Some(tx) = state.mempool.get(&hash).await {
        return Ok(Json(tx));
    }

    // Check committed transactions
    let chain_state = state.chain_state.read().await;
    if let Some(tx) = chain_state.get_transaction(&hash) {
        return Ok(Json(tx.clone()));
    }

    Err(RpcError::NotFound(format!(
        "Transaction {} not found",
        hash_hex
    )))
}

/// GET /account/:pubkey - Get account by public key
pub async fn get_account<S: Storage + Send + Sync>(
    State(state): State<Arc<AppState<S>>>,
    Path(pubkey_hex): Path<String>,
) -> Result<Json<AccountResponse>, RpcError> {
    let pubkey = PublicKey::from_hex(&pubkey_hex)
        .map_err(|_| RpcError::BadRequest("Invalid public key".to_string()))?;

    let chain_state = state.chain_state.read().await;

    let account = chain_state.get_account(&pubkey).cloned().unwrap_or_default();

    Ok(Json(AccountResponse {
        pubkey: pubkey_hex,
        balance: account.native_balance(),
        nonce: account.nonce,
        total_balance: account.total_native_balance(),
    }))
}

/// GET /block/:height - Get block by height
pub async fn get_block<S: Storage + Send + Sync>(
    State(state): State<Arc<AppState<S>>>,
    Path(height): Path<u64>,
) -> Result<Json<BlockResponse>, RpcError> {
    let chain_state = state.chain_state.read().await;

    if let Some(block) = chain_state.get_block(height) {
        let hash = block.hash().map_err(|e| RpcError::Internal(e.to_string()))?;
        return Ok(Json(BlockResponse {
            height: block.header.height,
            hash: hash.to_hex(),
            prev_hash: block.header.prev_hash.to_hex(),
            timestamp: block.header.timestamp,
            tx_count: block.txs.len(),
            proposer: block.header.proposer_pubkey.to_hex(),
            tx_root: block.header.tx_root.to_hex(),
            state_root: block.header.state_root.to_hex(),
        }));
    }

    Err(RpcError::NotFound(format!(
        "Block at height {} not found",
        height
    )))
}

/// GET /claim/:id - Get claim by ID
pub async fn get_claim<S: Storage + Send + Sync>(
    State(state): State<Arc<AppState<S>>>,
    Path(claim_id_hex): Path<String>,
) -> Result<Json<ClaimResponse>, RpcError> {
    let claim_id = Hash::from_hex(&claim_id_hex)
        .map_err(|_| RpcError::BadRequest("Invalid claim ID".to_string()))?;

    let chain_state = state.chain_state.read().await;

    let claim = chain_state
        .get_claim(&claim_id)
        .ok_or_else(|| RpcError::NotFound(format!("Claim {} not found", claim_id_hex)))?;

    let status = match claim.status {
        seloria_core::ClaimStatus::Pending => "pending",
        seloria_core::ClaimStatus::FinalizedYes => "finalized_yes",
        seloria_core::ClaimStatus::FinalizedNo => "finalized_no",
    };

    Ok(Json(ClaimResponse {
        id: claim_id_hex,
        claim_type: claim.claim_type.clone(),
        payload_hash: claim.payload_hash.to_hex(),
        creator: claim.creator.to_hex(),
        creator_stake: claim.creator_stake,
        yes_stake: claim.yes_stake,
        no_stake: claim.no_stake,
        status: status.to_string(),
        created_at: claim.created_at,
        attestation_count: claim.attestations.len(),
    }))
}

/// GET /kv/:ns_id/:key - Get KV entry
pub async fn get_kv<S: Storage + Send + Sync>(
    State(state): State<Arc<AppState<S>>>,
    Path((ns_id_hex, key)): Path<(String, String)>,
) -> Result<Json<KvResponse>, RpcError> {
    let ns_id = Hash::from_hex(&ns_id_hex)
        .map_err(|_| RpcError::BadRequest("Invalid namespace ID".to_string()))?;

    let chain_state = state.chain_state.read().await;

    let value = chain_state
        .kv_get(&ns_id, &key)
        .ok_or_else(|| RpcError::NotFound(format!("Key '{}' not found", key)))?;

    let data = match &value.data {
        seloria_core::KvData::Inline(bytes) => {
            // Try to parse as JSON, fall back to base64
            if value.codec == "json" {
                serde_json::from_slice(bytes).unwrap_or_else(|_| {
                    serde_json::Value::String(hex::encode(bytes))
                })
            } else {
                serde_json::Value::String(hex::encode(bytes))
            }
        }
        seloria_core::KvData::Reference { hash, uri } => {
            serde_json::json!({
                "type": "reference",
                "hash": hash.to_hex(),
                "uri": uri
            })
        }
    };

    Ok(Json(KvResponse {
        ns_id: ns_id_hex,
        key,
        codec: value.codec.clone(),
        data,
    }))
}

/// GET /kv/:ns_id - List keys in namespace
pub async fn list_kv_keys<S: Storage + Send + Sync>(
    State(state): State<Arc<AppState<S>>>,
    Path(ns_id_hex): Path<String>,
) -> Result<Json<KvKeysResponse>, RpcError> {
    let ns_id = Hash::from_hex(&ns_id_hex)
        .map_err(|_| RpcError::BadRequest("Invalid namespace ID".to_string()))?;

    let chain_state = state.chain_state.read().await;

    let keys = chain_state.kv_keys(&ns_id);

    Ok(Json(KvKeysResponse {
        ns_id: ns_id_hex,
        keys,
    }))
}

/// POST /cert/issue - Issue a signed agent certificate (dev-only)
pub async fn issue_certificate<S: Storage + Send + Sync>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<IssueCertRequest>,
) -> Result<Json<IssueCertResponse>, RpcError> {
    let issuer_lock = state
        .issuer_keypair
        .as_ref()
        .ok_or_else(|| RpcError::BadRequest("Issuer key not configured".to_string()))?;
    let issuer = issuer_lock.lock().await.clone();

    let agent_pubkey = PublicKey::from_hex(&request.agent_pubkey)
        .map_err(|_| RpcError::BadRequest("Invalid agent pubkey".to_string()))?;

    let issuer_id = seloria_core::hash_blake3(issuer.public.as_bytes());
    let metadata_hash = match request.metadata_hash {
        Some(h) => Hash::from_hex(&h)
            .map_err(|_| RpcError::BadRequest("Invalid metadata hash".to_string()))?,
        None => Hash::ZERO,
    };

    let cert = seloria_core::AgentCertificate::new(
        issuer_id,
        agent_pubkey,
        request.issued_at,
        request.expires_at,
        request.capabilities.clone(),
        metadata_hash,
    );
    let signed = seloria_core::SignedAgentCertificate::new(cert, &issuer.secret)?;

    Ok(Json(IssueCertResponse { cert: signed }))
}

/// POST /consensus/propose - Validate and sign a proposed block
pub async fn consensus_propose<S: Storage + Send + Sync + Clone>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<ProposeRequest>,
) -> Result<Json<ProposeResponse>, RpcError> {
    let keypair_lock = state
        .validator_keypair
        .as_ref()
        .ok_or_else(|| RpcError::BadRequest("Node is not a validator".to_string()))?;
    let keypair = keypair_lock.lock().await.clone();

    let chain_state = state.chain_state.read().await;
    if chain_state.validators.is_empty() {
        return Err(RpcError::BadRequest("No validators configured".to_string()));
    }

    // Check leader is correct for this height
    let leader_index = (request.block.header.height as usize) % chain_state.validators.len();
    let expected_leader = chain_state.validators[leader_index];
    if request.block.header.proposer_pubkey != expected_leader {
        return Err(RpcError::BadRequest(format!(
            "Unexpected proposer for height {}",
            request.block.header.height
        )));
    }

    let block_builder = BlockBuilder::new(BlockBuilderConfig {
        chain_id: chain_state.chain_id,
        ..Default::default()
    });
    let validator = Validator::new(keypair.public, keypair.secret.clone(), block_builder);

    let signature = validator
        .validate_and_sign(&request.block, &chain_state)
        .map_err(|e| RpcError::BadRequest(e.to_string()))?;

    Ok(Json(ProposeResponse {
        validator_pubkey: keypair.public,
        signature,
    }))
}

/// POST /consensus/commit - Commit a finalized block with QC
pub async fn consensus_commit<S: Storage + Send + Sync + Clone>(
    State(state): State<Arc<AppState<S>>>,
    Json(request): Json<CommitRequest>,
) -> Result<Json<CommitResponse>, RpcError> {
    let block = request.block;
    let qc = block
        .qc
        .as_ref()
        .ok_or_else(|| RpcError::BadRequest("Missing quorum certificate".to_string()))?;

    let block_hash = block
        .hash()
        .map_err(|e| RpcError::BadRequest(e.to_string()))?;

    if qc.block_hash != block_hash {
        return Err(RpcError::BadRequest(
            "QC block hash does not match block".to_string(),
        ));
    }

    let mut chain_state = state.chain_state.write().await;
    let validators = chain_state.validators.clone();
    if validators.is_empty() {
        return Err(RpcError::BadRequest("No validators configured".to_string()));
    }
    let threshold = (validators.len() * 2 / 3) + 1;

    verify_qc(qc, &validators, threshold)
        .map_err(|e| RpcError::BadRequest(e.to_string()))?;

    let block_builder = BlockBuilder::new(BlockBuilderConfig {
        chain_id: chain_state.chain_id,
        ..Default::default()
    });

    let results = block_builder
        .apply_block(&mut *chain_state, &block)
        .map_err(|e| RpcError::BadRequest(e.to_string()))?;
    chain_state
        .persist_state()
        .map_err(|e| RpcError::Internal(e.to_string()))?;
    drop(chain_state);

    // Remove committed transactions from mempool
    let tx_hashes: Vec<Hash> = block
        .txs
        .iter()
        .filter_map(|tx| tx.hash().ok())
        .collect();
    state.mempool.remove_committed(&tx_hashes).await;

    // Broadcast events
    state.broadcaster.broadcast(crate::ws::WsEvent::block_committed(
        block.header.height,
        block_hash,
        block.txs.len(),
        block.header.timestamp,
    ));

    for (tx, result) in block.txs.iter().zip(results.iter()) {
        state.broadcaster.broadcast(crate::ws::WsEvent::tx_applied(
            result.tx_hash,
            tx.sender_pubkey,
            result.success,
        ));

        for event in &result.events {
            if let Some(ws_event) = crate::ws::WsEvent::from_execution_event(event) {
                state.broadcaster.broadcast(ws_event);
            }
        }
    }

    Ok(Json(CommitResponse {
        status: "committed".to_string(),
        height: block.header.height,
        hash: block_hash.to_hex(),
    }))
}
