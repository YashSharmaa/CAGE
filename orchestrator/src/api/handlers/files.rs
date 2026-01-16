//! File management handlers

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use serde::Deserialize;

use crate::api::{ApiError, UserAuth};
use crate::models::{FileListResponse, FileUploadRequest, FileUploadResponse};
use crate::AppState;

/// Query parameters for file listing
#[derive(Debug, Deserialize)]
pub struct ListFilesQuery {
    #[serde(default = "default_path")]
    pub path: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub recursive: bool,
}

fn default_path() -> String {
    "/".to_string()
}

/// List files in workspace
pub async fn list_files(
    State(state): State<Arc<AppState>>,
    auth: UserAuth,
    Query(query): Query<ListFilesQuery>,
) -> Result<Json<FileListResponse>, ApiError> {
    let response = state
        .container_manager
        .list_files(&auth.user_id, &query.path)
        .await?;

    Ok(Json(response))
}

/// Upload a file (supports both multipart and JSON)
pub async fn upload_file(
    State(state): State<Arc<AppState>>,
    auth: UserAuth,
    body: UploadBody,
) -> Result<Json<FileUploadResponse>, ApiError> {
    let (filename, path, contents) = match body {
        UploadBody::Json(req) => {
            let contents = base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                &req.content,
            )
            .map_err(|_| ApiError::BadRequest("Invalid base64 content".into()))?;
            (req.filename, req.path, contents)
        }
        UploadBody::Multipart { filename, path, data } => {
            (filename, path.unwrap_or_else(|| "/".into()), data)
        }
    };

    // Validate filename
    if filename.is_empty() || filename.contains("..") || filename.contains('/') {
        return Err(ApiError::BadRequest("Invalid filename".into()));
    }

    // Check size limit (100MB default)
    if contents.len() > 100 * 1024 * 1024 {
        return Err(ApiError::PayloadTooLarge);
    }

    let filepath = if path == "/" {
        filename.clone()
    } else {
        format!("{}/{}", path.trim_matches('/'), filename)
    };

    let checksum = state
        .container_manager
        .write_file(&auth.user_id, &filepath, &contents)
        .await?;

    Ok(Json(FileUploadResponse {
        path: format!("/{}", filepath),
        size_bytes: contents.len() as u64,
        checksum,
    }))
}

/// Download a file
pub async fn download_file(
    State(state): State<Arc<AppState>>,
    auth: UserAuth,
    Path(filepath): Path<String>,
) -> Result<Response, ApiError> {
    let contents = state
        .container_manager
        .read_file(&auth.user_id, &filepath)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") || e.to_string().contains("No such file") {
                ApiError::NotFound(format!("File not found: {}", filepath))
            } else {
                ApiError::from(e)
            }
        })?;

    // Determine content type from extension
    let content_type = mime_guess::from_path(&filepath)
        .first_or_octet_stream()
        .to_string();

    let filename = filepath.split('/').next_back().unwrap_or(&filepath);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(Body::from(contents))
        .unwrap())
}

/// Delete a file
pub async fn delete_file(
    State(state): State<Arc<AppState>>,
    auth: UserAuth,
    Path(filepath): Path<String>,
) -> Result<StatusCode, ApiError> {
    state
        .container_manager
        .delete_file(&auth.user_id, &filepath)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") || e.to_string().contains("No such file") {
                ApiError::NotFound(format!("File not found: {}", filepath))
            } else {
                ApiError::from(e)
            }
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Upload body that supports both JSON and multipart
pub enum UploadBody {
    Json(FileUploadRequest),
    Multipart {
        filename: String,
        path: Option<String>,
        data: Vec<u8>,
    },
}

use axum::extract::FromRequest;

#[axum::async_trait]
impl<S> FromRequest<S> for UploadBody
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(
        req: axum::extract::Request,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if content_type.starts_with("multipart/form-data") {
            let mut multipart = Multipart::from_request(req, state)
                .await
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;

            let mut filename = None;
            let mut path = None;
            let mut data = None;

            while let Some(field) = multipart
                .next_field()
                .await
                .map_err(|e| ApiError::BadRequest(e.to_string()))?
            {
                let name = field.name().unwrap_or("").to_string();
                match name.as_str() {
                    "file" => {
                        filename = field.file_name().map(|s| s.to_string());
                        data = Some(
                            field
                                .bytes()
                                .await
                                .map_err(|e| ApiError::BadRequest(e.to_string()))?
                                .to_vec(),
                        );
                    }
                    "path" => {
                        path = Some(
                            field
                                .text()
                                .await
                                .map_err(|e| ApiError::BadRequest(e.to_string()))?,
                        );
                    }
                    _ => {}
                }
            }

            let filename = filename.ok_or_else(|| ApiError::BadRequest("Missing filename".into()))?;
            let data = data.ok_or_else(|| ApiError::BadRequest("Missing file data".into()))?;

            Ok(UploadBody::Multipart {
                filename,
                path,
                data,
            })
        } else {
            let Json(body) = Json::<FileUploadRequest>::from_request(req, state)
                .await
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;
            Ok(UploadBody::Json(body))
        }
    }
}
