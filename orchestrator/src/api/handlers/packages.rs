//! Package management API endpoints
//!
//! Endpoints for installing and listing packages in sandboxes

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, UserAuth};
use crate::models::Language;
use crate::AppState;

/// Request to install a package
#[derive(Debug, Deserialize)]
pub struct InstallPackageRequest {
    pub package: String,
    pub language: Language,
}

/// Response from package installation
#[derive(Debug, Serialize)]
pub struct InstallPackageResponse {
    pub package: String,
    pub language: String,
    pub status: String,
    pub output: String,
}

/// List of installed packages
#[derive(Debug, Serialize)]
pub struct InstalledPackagesResponse {
    pub packages: Vec<String>,
    pub total: usize,
}

/// List of allowed packages
#[derive(Debug, Serialize)]
pub struct AllowedPackagesResponse {
    pub language: String,
    pub packages: Vec<String>,
    pub total: usize,
}

/// Install a package
pub async fn install_package(
    State(state): State<Arc<AppState>>,
    UserAuth { user_id, .. }: UserAuth,
    Json(request): Json<InstallPackageRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Check if package installation is enabled
    if !state.config.packages.enabled {
        return Err(ApiError::Forbidden);
    }

    // Get user's session
    let session = state
        .container_manager
        .get_session_handle(&user_id)
        .await
        .ok_or_else(|| ApiError::NotFound("No active session".to_string()))?;

    let container_id = session
        .container_id()
        .await
        .ok_or_else(|| ApiError::Internal("No container ID".to_string()))?;

    // Install the package
    let output = state
        .package_manager
        .install_package(
            &user_id,
            &container_id,
            &request.package,
            request.language,
            state.container_manager.podman_path(),
        )
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(InstallPackageResponse {
            package: request.package,
            language: request.language.as_str().to_string(),
            status: "installed".to_string(),
            output,
        }),
    ))
}

/// List installed packages for current user
pub async fn list_installed(
    State(state): State<Arc<AppState>>,
    UserAuth { user_id, .. }: UserAuth,
) -> Result<impl IntoResponse, ApiError> {
    let packages = state.package_manager.list_installed(&user_id).await;

    Ok((
        StatusCode::OK,
        Json(InstalledPackagesResponse {
            total: packages.len(),
            packages,
        }),
    ))
}

/// List allowed packages for a language
pub async fn list_allowed(
    State(state): State<Arc<AppState>>,
    Path(language_str): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let language = match language_str.to_lowercase().as_str() {
        "python" => Language::Python,
        "javascript" | "js" => Language::Javascript,
        "r" => Language::R,
        _ => return Err(ApiError::BadRequest(format!("Unsupported language: {}", language_str))),
    };

    let packages = state.package_manager.get_allowlist(language).await;

    Ok((
        StatusCode::OK,
        Json(AllowedPackagesResponse {
            language: language.as_str().to_string(),
            total: packages.len(),
            packages,
        }),
    ))
}
