use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use seloria_state::Storage;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use super::handlers::{
    consensus_commit, consensus_propose, get_account, get_block, get_claim, get_kv,
    faucet, get_snapshot, get_snapshot_meta, get_status, get_tx, issue_certificate,
    list_kv_keys, publish_snapshot, submit_tx, AppState,
};

/// Create the HTTP router
pub fn create_router<S: Storage + Send + Sync + Clone + 'static>(
    state: Arc<AppState<S>>,
) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/status", get(get_status::<S>))
        .route("/snapshot/meta", get(get_snapshot_meta::<S>))
        .route("/snapshot", get(get_snapshot::<S>))
        .route("/snapshot/publish", post(publish_snapshot::<S>))
        .route("/tx", post(submit_tx::<S>))
        .route("/faucet", post(faucet::<S>))
        .route("/tx/{hash}", get(get_tx::<S>))
        .route("/account/{pubkey}", get(get_account::<S>))
        .route("/block/{height}", get(get_block::<S>))
        .route("/claim/{id}", get(get_claim::<S>))
        .route("/kv/{ns_id}", get(list_kv_keys::<S>))
        .route("/kv/{ns_id}/{key}", get(get_kv::<S>))
        .route("/cert/issue", post(issue_certificate::<S>))
        .route("/consensus/propose", post(consensus_propose::<S>))
        .route("/consensus/commit", post(consensus_commit::<S>))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}
