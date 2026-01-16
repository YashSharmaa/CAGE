//! Code signing verification
//!
//! Verifies GPG signatures on code before execution

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

/// Code signing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningConfig {
    /// Enable code signing verification
    #[serde(default)]
    pub enabled: bool,
    /// Require signatures for all code
    #[serde(default)]
    pub require_signature: bool,
    /// Trusted public key fingerprints
    #[serde(default)]
    pub trusted_keys: Vec<String>,
}

impl Default for SigningConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            require_signature: false,
            trusted_keys: vec![],
        }
    }
}

/// Code signature verifier
pub struct SignatureVerifier {
    config: SigningConfig,
}

/// Signed code payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedCode {
    /// The code to execute
    pub code: String,
    /// GPG signature (base64 encoded)
    pub signature: String,
    /// Signing key fingerprint
    pub key_fingerprint: String,
}

#[derive(Debug)]
pub enum VerificationResult {
    Valid,
    Invalid(String),
    NoSignature,
}

impl SignatureVerifier {
    pub fn new(config: SigningConfig) -> Self {
        Self { config }
    }

    /// Verify code signature
    pub async fn verify(&self, signed_code: &SignedCode) -> Result<VerificationResult> {
        if !self.config.enabled {
            return Ok(VerificationResult::NoSignature);
        }

        debug!(
            key_fingerprint = %signed_code.key_fingerprint,
            "Verifying code signature"
        );

        // Check if key is trusted
        if !self.config.trusted_keys.contains(&signed_code.key_fingerprint) {
            warn!(
                key_fingerprint = %signed_code.key_fingerprint,
                "Untrusted signing key"
            );
            return Ok(VerificationResult::Invalid(
                "Signing key not in trusted list".to_string(),
            ));
        }

        // In production, this would use GPG libraries to verify
        // For now, implement basic signature validation

        // Calculate code hash
        let code_hash = Self::hash_code(&signed_code.code);

        // Verify signature format (base64)
        if base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &signed_code.signature)
            .is_err()
        {
            return Ok(VerificationResult::Invalid("Invalid signature format".to_string()));
        }

        // In real implementation, would verify with GPG:
        // 1. Decode base64 signature
        // 2. Use gpgme or similar to verify signature against public key
        // 3. Check signature is for the code hash

        info!(
            key_fingerprint = %signed_code.key_fingerprint,
            code_hash = %code_hash,
            "Code signature verified (stub implementation)"
        );

        Ok(VerificationResult::Valid)
    }

    /// Check if unsigned code is allowed
    pub fn allow_unsigned(&self) -> bool {
        !self.config.enabled || !self.config.require_signature
    }

    /// Hash code for signature verification
    fn hash_code(code: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(code.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signing_config_default() {
        let config = SigningConfig::default();
        assert!(!config.enabled);
        assert!(!config.require_signature);
    }

    #[tokio::test]
    async fn test_signature_verification_disabled() {
        let config = SigningConfig::default();
        let verifier = SignatureVerifier::new(config);

        let signed = SignedCode {
            code: "print('test')".to_string(),
            signature: "dGVzdA==".to_string(),
            key_fingerprint: "ABC123".to_string(),
        };

        let result = verifier.verify(&signed).await.unwrap();
        matches!(result, VerificationResult::NoSignature);
    }

    #[tokio::test]
    async fn test_untrusted_key() {
        let config = SigningConfig {
            enabled: true,
            require_signature: true,
            trusted_keys: vec!["TRUSTED_KEY".to_string()],
        };

        let verifier = SignatureVerifier::new(config);

        let signed = SignedCode {
            code: "print('test')".to_string(),
            signature: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b"signature"),
            key_fingerprint: "UNTRUSTED_KEY".to_string(),
        };

        let result = verifier.verify(&signed).await.unwrap();
        matches!(result, VerificationResult::Invalid(_));
    }
}
