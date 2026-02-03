use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use super::events::{EventBroadcaster, WsEvent};

/// WebSocket server state
pub struct WsState {
    pub broadcaster: Arc<EventBroadcaster>,
}

/// Create WebSocket router
pub fn create_ws_router(broadcaster: Arc<EventBroadcaster>) -> Router {
    let state = Arc::new(WsState { broadcaster });

    Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state)
}

/// WebSocket upgrade handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WsState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle an individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<WsState>) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to events
    let mut event_rx = state.broadcaster.subscribe();

    info!("New WebSocket connection");

    // Send events to client
    let send_task = tokio::spawn(async move {
        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    let json = match serde_json::to_string(&event) {
                        Ok(j) => j,
                        Err(e) => {
                            error!("Failed to serialize event: {}", e);
                            continue;
                        }
                    };

                    if let Err(e) = sender.send(Message::Text(json.into())).await {
                        warn!("Failed to send WebSocket message: {}", e);
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("WebSocket client lagged {} events", n);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    debug!("Event channel closed");
                    break;
                }
            }
        }
    });

    // Handle incoming messages (for keep-alive pings)
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Ping(data)) => {
                    debug!("Received ping");
                    // Pong is sent automatically by axum
                }
                Ok(Message::Close(_)) => {
                    debug!("Client closed connection");
                    break;
                }
                Ok(Message::Text(text)) => {
                    // Could handle subscription filtering here
                    debug!("Received message: {}", text);
                }
                Err(e) => {
                    warn!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }

    info!("WebSocket connection closed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_router() {
        let broadcaster = Arc::new(EventBroadcaster::new(100));
        let _router = create_ws_router(broadcaster);
    }
}
