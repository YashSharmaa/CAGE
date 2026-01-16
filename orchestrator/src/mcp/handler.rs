//! MCP WebSocket handler

use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use futures::{SinkExt, StreamExt};
use serde_json::json;
use tracing::{debug, error, info};

use crate::AppState;

use super::{methods, MCPRequest, MCPResponse};

/// Handle MCP WebSocket upgrade
pub async fn mcp_websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_mcp_socket(socket, state))
}

/// Handle MCP WebSocket connection
async fn handle_mcp_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    info!("MCP WebSocket connection established");

    // Session context for this connection
    let mut user_id: Option<String> = None;
    let mut initialized = false;

    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                debug!("MCP message received: {}", text);

                let request: Result<MCPRequest, _> = serde_json::from_str(&text);

                let response = match request {
                    Ok(req) => {
                        handle_mcp_request(req, &mut user_id, &mut initialized, &state).await
                    }
                    Err(e) => MCPResponse::error(
                        None,
                        -32700,
                        format!("Parse error: {}", e),
                    ),
                };

                let response_text = serde_json::to_string(&response).unwrap_or_else(|_| {
                    r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Internal error"}}"#
                        .to_string()
                });

                if sender.send(Message::Text(response_text)).await.is_err() {
                    break;
                }
            }
            Ok(Message::Close(_)) => {
                info!("MCP WebSocket connection closed");
                break;
            }
            Ok(Message::Ping(data)) => {
                if sender.send(Message::Pong(data)).await.is_err() {
                    break;
                }
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
}

/// Handle individual MCP request
async fn handle_mcp_request(
    request: MCPRequest,
    user_id: &mut Option<String>,
    initialized: &mut bool,
    state: &Arc<AppState>,
) -> MCPResponse {
    match request.method.as_str() {
        methods::INITIALIZE => {
            *initialized = true;

            // Extract user from params
            if let Some(params) = &request.params {
                if let Some(uid) = params.get("user_id").and_then(|v| v.as_str()) {
                    *user_id = Some(uid.to_string());
                }
            }

            MCPResponse::success(
                request.id,
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {},
                        "resources": {}
                    },
                    "serverInfo": {
                        "name": "CAGE",
                        "version": "1.0.0"
                    }
                }),
            )
        }

        methods::LIST_TOOLS => {
            if !*initialized {
                return MCPResponse::error(request.id, -32002, "Not initialized".to_string());
            }

            let tools = super::get_mcp_tools();
            MCPResponse::success(request.id, json!({ "tools": tools }))
        }

        methods::CALL_TOOL => {
            if !*initialized {
                return MCPResponse::error(request.id, -32002, "Not initialized".to_string());
            }

            let uid = match user_id {
                Some(u) => u.clone(),
                None => return MCPResponse::error(request.id, -32001, "No user authenticated".to_string()),
            };

            handle_tool_call(request, &uid, state).await
        }

        methods::LIST_RESOURCES => {
            if !*initialized {
                return MCPResponse::error(request.id, -32002, "Not initialized".to_string());
            }

            let uid = user_id.as_deref().unwrap_or("default");
            let resources = super::get_mcp_resources(uid);
            MCPResponse::success(request.id, json!({ "resources": resources }))
        }

        _ => MCPResponse::error(request.id, -32601, format!("Method not found: {}", request.method)),
    }
}

/// Handle MCP tool call
async fn handle_tool_call(
    request: MCPRequest,
    user_id: &str,
    state: &Arc<AppState>,
) -> MCPResponse {
    let params = match &request.params {
        Some(p) => p,
        None => return MCPResponse::error(request.id, -32602, "Invalid params".to_string()),
    };

    let tool_name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => return MCPResponse::error(request.id, -32602, "Missing tool name".to_string()),
    };

    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    match tool_name {
        "execute_code" => {
            let code = match arguments.get("code").and_then(|v| v.as_str()) {
                Some(c) => c.to_string(),
                None => return MCPResponse::error(request.id, -32602, "Missing code".to_string()),
            };

            let language = arguments
                .get("language")
                .and_then(|v| v.as_str())
                .unwrap_or("python");

            let persistent = arguments
                .get("persistent")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let exec_request = crate::models::ExecuteRequest {
                language: match language {
                    "javascript" => crate::models::Language::Javascript,
                    "bash" => crate::models::Language::Bash,
                    _ => crate::models::Language::Python,
                },
                code,
                timeout_seconds: arguments
                    .get("timeout_seconds")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(30),
                working_dir: None,
                env: std::collections::HashMap::new(),
                persistent,
            };

            match state.container_manager.execute_code(user_id, exec_request).await {
                Ok(result) => {
                    MCPResponse::success(request.id, json!({
                        "content": [{
                            "type": "text",
                            "text": result.stdout
                        }],
                        "isError": result.status != crate::models::ExecutionStatus::Success,
                        "metadata": {
                            "execution_id": result.execution_id,
                            "duration_ms": result.duration_ms,
                            "files_created": result.files_created
                        }
                    }))
                }
                Err(e) => MCPResponse::error(request.id, -32000, e.to_string()),
            }
        }

        "list_files" => {
            let path = arguments.get("path").and_then(|v| v.as_str()).unwrap_or("/");

            match state.container_manager.list_files(user_id, path).await {
                Ok(file_list) => {
                    MCPResponse::success(request.id, json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string_pretty(&file_list).unwrap()
                        }]
                    }))
                }
                Err(e) => MCPResponse::error(request.id, -32000, e.to_string()),
            }
        }

        "upload_file" => {
            let filename = match arguments.get("filename").and_then(|v| v.as_str()) {
                Some(f) => f,
                None => return MCPResponse::error(request.id, -32602, "Missing filename".to_string()),
            };

            let content_b64 = match arguments.get("content").and_then(|v| v.as_str()) {
                Some(c) => c,
                None => return MCPResponse::error(request.id, -32602, "Missing content".to_string()),
            };

            let contents = match base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                content_b64,
            ) {
                Ok(c) => c,
                Err(_) => return MCPResponse::error(request.id, -32602, "Invalid base64".to_string()),
            };

            match state.container_manager.write_file(user_id, filename, &contents).await {
                Ok(checksum) => {
                    MCPResponse::success(request.id, json!({
                        "content": [{
                            "type": "text",
                            "text": format!("File uploaded: {} (checksum: {})", filename, checksum)
                        }]
                    }))
                }
                Err(e) => MCPResponse::error(request.id, -32000, e.to_string()),
            }
        }

        _ => MCPResponse::error(request.id, -32601, format!("Unknown tool: {}", tool_name)),
    }
}
