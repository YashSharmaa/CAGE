//! Model Context Protocol (MCP) implementation for CAGE
//!
//! Provides JSON-RPC 2.0 interface over WebSocket for LLM integration

pub mod handler;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// MCP JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

/// MCP JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<MCPError>,
}

/// MCP Error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl MCPResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<Value>, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(MCPError {
                code,
                message,
                data: None,
            }),
        }
    }
}

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// MCP Resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPResource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: Option<String>,
}

/// MCP method names
pub mod methods {
    pub const INITIALIZE: &str = "initialize";
    pub const LIST_TOOLS: &str = "tools/list";
    pub const CALL_TOOL: &str = "tools/call";
    pub const LIST_RESOURCES: &str = "resources/list";
    pub const READ_RESOURCE: &str = "resources/read";
    pub const EXECUTE_CODE: &str = "cage/execute";
    pub const LIST_FILES: &str = "cage/files/list";
    pub const UPLOAD_FILE: &str = "cage/files/upload";
    pub const DOWNLOAD_FILE: &str = "cage/files/download";
    pub const GET_SESSION: &str = "cage/session/get";
    pub const TERMINATE_SESSION: &str = "cage/session/terminate";
}

/// Get list of MCP tools exposed by CAGE
pub fn get_mcp_tools() -> Vec<MCPTool> {
    vec![
        MCPTool {
            name: "execute_code".to_string(),
            description: "Execute code in a secure sandbox environment".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "language": {
                        "type": "string",
                        "enum": ["python", "javascript", "bash"],
                        "description": "Programming language"
                    },
                    "code": {
                        "type": "string",
                        "description": "Code to execute"
                    },
                    "timeout_seconds": {
                        "type": "integer",
                        "description": "Maximum execution time",
                        "default": 30
                    },
                    "persistent": {
                        "type": "boolean",
                        "description": "Use persistent interpreter (maintains state)",
                        "default": false
                    }
                },
                "required": ["code"]
            }),
        },
        MCPTool {
            name: "list_files".to_string(),
            description: "List files in sandbox workspace".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory path",
                        "default": "/"
                    }
                }
            }),
        },
        MCPTool {
            name: "upload_file".to_string(),
            description: "Upload a file to sandbox workspace".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "filename": {
                        "type": "string",
                        "description": "Target filename"
                    },
                    "content": {
                        "type": "string",
                        "description": "Base64 encoded file content"
                    }
                },
                "required": ["filename", "content"]
            }),
        },
        MCPTool {
            name: "download_file".to_string(),
            description: "Download a file from sandbox workspace".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "filename": {
                        "type": "string",
                        "description": "File to download"
                    }
                },
                "required": ["filename"]
            }),
        },
    ]
}

/// Get list of MCP resources
pub fn get_mcp_resources(user_id: &str) -> Vec<MCPResource> {
    vec![
        MCPResource {
            uri: format!("cage://sessions/{}", user_id),
            name: format!("{}'s sandbox session", user_id),
            description: "Current sandbox session information".to_string(),
            mime_type: Some("application/json".to_string()),
        },
        MCPResource {
            uri: format!("cage://files/{}", user_id),
            name: format!("{}'s workspace files", user_id),
            description: "Files in user's sandbox workspace".to_string(),
            mime_type: Some("application/json".to_string()),
        },
    ]
}
