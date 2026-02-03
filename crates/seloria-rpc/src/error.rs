use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RpcError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Mempool error: {0}")]
    Mempool(#[from] seloria_mempool::MempoolError),

    #[error("Core error: {0}")]
    Core(#[from] seloria_core::CoreError),

    #[error("State error: {0}")]
    State(#[from] seloria_state::StateError),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl IntoResponse for RpcError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            RpcError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            RpcError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            RpcError::Transaction(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            RpcError::Mempool(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            RpcError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            RpcError::Core(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            RpcError::State(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            RpcError::Serialization(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
        };

        let body = json!({
            "error": message
        });

        (status, axum::Json(body)).into_response()
    }
}
