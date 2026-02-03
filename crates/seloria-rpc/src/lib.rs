//! Seloria RPC - HTTP and WebSocket API
//!
//! This crate provides the RPC layer for the Seloria node.

pub mod error;
pub mod http;
pub mod ws;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use seloria_mempool::Mempool;
use seloria_state::{ChainState, Storage};
use tokio::sync::RwLock;
use tracing::info;

use http::{create_router, AppState};
use ws::{create_ws_router, EventBroadcaster};

pub use error::RpcError;
pub use http::handlers::{AccountResponse, BlockResponse, ClaimResponse, StatusResponse};
pub use ws::WsEvent;

/// RPC server configuration
#[derive(Debug, Clone)]
pub struct RpcConfig {
    /// HTTP bind address
    pub http_addr: SocketAddr,
    /// Enable WebSocket
    pub enable_ws: bool,
}

impl Default for RpcConfig {
    fn default() -> Self {
        RpcConfig {
            http_addr: "127.0.0.1:8080".parse().unwrap(),
            enable_ws: true,
        }
    }
}

/// RPC server
pub struct RpcServer<S: Storage> {
    config: RpcConfig,
    app_state: Arc<AppState<S>>,
}

impl<S: Storage + Send + Sync + Clone + 'static> RpcServer<S> {
    /// Create a new RPC server
    pub fn new(
        config: RpcConfig,
        chain_state: Arc<RwLock<ChainState<S>>>,
        mempool: Arc<Mempool>,
        broadcaster: Arc<EventBroadcaster>,
        validator_keypair: Option<Arc<tokio::sync::Mutex<seloria_core::KeyPair>>>,
        issuer_keypair: Option<Arc<tokio::sync::Mutex<seloria_core::KeyPair>>>,
        snapshot_path: Option<std::path::PathBuf>,
        validator_endpoints: Vec<seloria_consensus::ValidatorEndpoint>,
        faucet_keypair: Option<Arc<tokio::sync::Mutex<seloria_core::KeyPair>>>,
    ) -> Self {
        let app_state = Arc::new(AppState {
            chain_state,
            mempool,
            broadcaster,
            validator_keypair,
            issuer_keypair,
            snapshot_path,
            validator_endpoints,
            faucet_keypair,
            faucet_lock: Arc::new(tokio::sync::Mutex::new(())),
        });

        RpcServer { config, app_state }
    }

    /// Get the event broadcaster
    pub fn broadcaster(&self) -> Arc<EventBroadcaster> {
        Arc::clone(&self.app_state.broadcaster)
    }

    /// Create the combined router
    pub fn router(&self) -> Router {
        let http_router = create_router(Arc::clone(&self.app_state));

        if self.config.enable_ws {
            let ws_router = create_ws_router(Arc::clone(&self.app_state.broadcaster));
            http_router.merge(ws_router)
        } else {
            http_router
        }
    }

    /// Run the RPC server
    pub async fn run(self) -> Result<(), std::io::Error> {
        let router = self.router();
        let addr = self.config.http_addr;

        info!("Starting RPC server on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, router).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use seloria_mempool::MempoolConfig;
    use seloria_state::MemoryStorage;

    #[test]
    fn test_rpc_server_creation() {
        let chain_state = Arc::new(RwLock::new(ChainState::new(MemoryStorage::new())));
        let mempool = Arc::new(Mempool::new(MempoolConfig::default()));
        let broadcaster = Arc::new(EventBroadcaster::default());

        let _server = RpcServer::new(
            RpcConfig::default(),
            chain_state,
            mempool,
            broadcaster,
            None,
            None,
            None,
            Vec::new(),
            None,
        );
    }
}
