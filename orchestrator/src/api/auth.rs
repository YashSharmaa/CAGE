//! Authentication and authorization middleware

use std::sync::Arc;

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::AppState;

use super::ApiError;

/// JWT claims for user authentication
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Expiration time (Unix timestamp)
    pub exp: u64,
    /// Issued at (Unix timestamp)
    pub iat: u64,
    /// Is admin user
    #[serde(default)]
    pub admin: bool,
}

/// Authenticated user extracted from request
#[derive(Debug, Clone)]
pub struct UserAuth {
    pub user_id: String,
    pub is_admin: bool,
}

/// Admin-only authentication
#[derive(Debug, Clone)]
pub struct AdminAuth {
    pub user_id: String,
}

#[async_trait]
impl FromRequestParts<Arc<AppState>> for UserAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        // Extract authorization header
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or(ApiError::Unauthorized)?;

        // Support both Bearer token and API key
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            // JWT token
            let claims = decode_jwt(token, &state.config.security.jwt_secret)?;
            Ok(UserAuth {
                user_id: claims.sub,
                is_admin: claims.admin,
            })
        } else if let Some(api_key) = auth_header.strip_prefix("ApiKey ") {
            // API key authentication
            let user_id = validate_api_key(api_key, state)?;
            let is_admin = state.config.is_admin(&user_id);
            Ok(UserAuth { user_id, is_admin })
        } else {
            // Check X-API-Key header
            if let Some(api_key) = parts.headers.get("X-API-Key").and_then(|h| h.to_str().ok()) {
                let user_id = validate_api_key(api_key, state)?;
                let is_admin = state.config.is_admin(&user_id);
                Ok(UserAuth { user_id, is_admin })
            } else {
                Err(ApiError::Unauthorized)
            }
        }
    }
}

#[async_trait]
impl FromRequestParts<Arc<AppState>> for AdminAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let user_auth = UserAuth::from_request_parts(parts, state).await?;

        if !user_auth.is_admin {
            return Err(ApiError::Forbidden);
        }

        Ok(AdminAuth {
            user_id: user_auth.user_id,
        })
    }
}

/// Decode and validate a JWT token
fn decode_jwt(token: &str, secret: &str) -> Result<Claims, ApiError> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    let validation = Validation::default();

    decode::<Claims>(token, &key, &validation)
        .map(|data| data.claims)
        .map_err(|e| {
            tracing::debug!(error = %e, "JWT validation failed");
            ApiError::Unauthorized
        })
}

/// Create a JWT token for a user
#[allow(dead_code)]
pub fn create_jwt(user_id: &str, is_admin: bool, secret: &str, expiration_secs: u64) -> String {
    let now = chrono::Utc::now().timestamp() as u64;
    let claims = Claims {
        sub: user_id.to_string(),
        exp: now + expiration_secs,
        iat: now,
        admin: is_admin,
    };

    let key = EncodingKey::from_secret(secret.as_bytes());
    encode(&Header::default(), &claims, &key).expect("JWT encoding should not fail")
}

/// Validate an API key and return the user ID
fn validate_api_key(api_key: &str, state: &AppState) -> Result<String, ApiError> {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};

    // Check admin token first
    if let Some(ref admin_token) = state.config.security.admin_token {
        if api_key == admin_token {
            return Ok("admin".to_string());
        }
    }

    // Check user API keys
    for (user_id, user_config) in &state.config.users {
        if let Some(ref hash) = user_config.api_key_hash {
            // Verify using argon2
            if let Ok(parsed_hash) = PasswordHash::new(hash) {
                if Argon2::default()
                    .verify_password(api_key.as_bytes(), &parsed_hash)
                    .is_ok()
                {
                    if !user_config.enabled {
                        return Err(ApiError::Forbidden);
                    }
                    return Ok(user_id.clone());
                }
            }
        }
    }

    // For development: allow any user if no users configured
    if state.config.users.is_empty() {
        // Extract user ID from a simple format: dev_<id>
        if let Some(user_id) = api_key.strip_prefix("dev_") {
            return Ok(user_id.to_string());
        }
    }

    Err(ApiError::Unauthorized)
}

/// Hash an API key for storage
#[allow(dead_code)]
pub fn hash_api_key(api_key: &str) -> Result<String, ApiError> {
    use argon2::{
        password_hash::{rand_core::OsRng, SaltString},
        Argon2, PasswordHasher,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(api_key.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| ApiError::Internal(format!("Failed to hash API key: {}", e)))
}
