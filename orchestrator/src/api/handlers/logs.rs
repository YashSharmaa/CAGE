//! Real-time log streaming via WebSocket

use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade, Query,
    },
    response::Response,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tracing::{error, info};

use crate::api::AdminAuth;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct LogStreamQuery {
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default = "default_level")]
    pub level: String,
}

fn default_level() -> String {
    "info".to_string()
}

/// WebSocket handler for real-time log streaming
pub async fn log_stream_handler(
    ws: WebSocketUpgrade,
    _admin: AdminAuth,
    Query(query): Query<LogStreamQuery>,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_log_stream(socket, query, state))
}

/// Handle log stream WebSocket connection
async fn handle_log_stream(
    socket: WebSocket,
    query: LogStreamQuery,
    _state: Arc<AppState>,
) {
    let (mut sender, mut receiver) = socket.split();

    info!(
        user_id = ?query.user_id,
        level = %query.level,
        "Log stream WebSocket connected"
    );

    // Send welcome message
    let welcome = serde_json::json!({
        "type": "connected",
        "message": "Log stream connected",
        "filters": {
            "user_id": query.user_id,
            "level": query.level,
        }
    });

    if sender
        .send(Message::Text(serde_json::to_string(&welcome).unwrap()))
        .await
        .is_err()
    {
        return;
    }

    // In a real implementation, this would:
    // 1. Subscribe to a log broadcast channel
    // 2. Filter logs based on query parameters
    // 3. Stream them to the WebSocket

    // For now, send periodic status messages
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let status_msg = serde_json::json!({
                    "type": "status",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "message": "Log streaming active. In production, logs would stream here in real-time.",
                });

                if sender.send(Message::Text(serde_json::to_string(&status_msg).unwrap())).await.is_err() {
                    break;
                }
            }

            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => {
                        info!("Log stream WebSocket closed");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}
