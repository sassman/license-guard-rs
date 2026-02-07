//! Direct verification API for advanced use cases.
//!
//! Use this when you need multiple verifiers (different products),
//! testable code without global state, or custom verification logic.
//!
//! For most apps, prefer the [`global`](crate::global) module.
//!
//! # Choosing a Verification Method
//!
//! Methods are layered — each adds a check on top of the previous:
//!
//! | Method | Signature | Expiry | Entitlement |
//! |--------|-----------|--------|-------------|
//! | [`verify`](LicenseVerifier::verify) | yes | — | — |
//! | [`verify_active`](LicenseVerifier::verify_active) | yes | yes | — |
//! | [`verify_with_entitlement`](LicenseVerifier::verify_with_entitlement) | yes | yes | yes |
//!
//! [`verify_auto`](LicenseVerifier::verify_auto) is a format-detecting
//! wrapper around [`verify`](LicenseVerifier::verify) — use it when the
//! input may be JSON or compact format (e.g. loaded from a file).
//!
//! Expiry checks compare against the system clock (UTC). Ensure the
//! host machine's clock is reasonably accurate.

use crate::error::LicenseError;
use crate::license::{LicenseFile, LicensePayload};
use base64::prelude::*;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

/// Verifies Ed25519-signed licenses against a public key.
///
/// Create one instance per product at startup and reuse it. The verifier
/// is immutable and can be shared across threads (`Send + Sync`).
///
/// # Example
///
/// ```rust,ignore
/// use license_guard::LicenseVerifier;
///
/// let verifier = LicenseVerifier::from_hex("abc123...")?;
/// let payload = verifier.verify_active(license_data)?;
/// println!("Licensed to: {}", payload.sub);
/// ```
pub struct LicenseVerifier {
    public_key: VerifyingKey,
}

impl LicenseVerifier {
    /// Create from hex-encoded public key.
    pub fn from_hex(hex_key: &str) -> Result<Self, LicenseError> {
        let bytes = hex::decode(hex_key)
            .map_err(|e| LicenseError::InvalidPublicKey(e.to_string()))?;
        Self::from_bytes(&bytes)
    }

    /// Create from raw 32-byte public key.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, LicenseError> {
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| LicenseError::InvalidPublicKey("key must be 32 bytes".into()))?;
        let public_key = VerifyingKey::from_bytes(&bytes)
            .map_err(|e| LicenseError::InvalidPublicKey(e.to_string()))?;
        Ok(Self { public_key })
    }

    /// Verify the Ed25519 signature and decode the payload.
    ///
    /// This is the lowest-level verify method — it checks only the
    /// cryptographic signature. It does **not** check expiry or
    /// entitlements. Use [`verify_active`](Self::verify_active) or
    /// [`verify_with_entitlement`](Self::verify_with_entitlement) for that.
    ///
    /// Expects `license_data` in JSON format. For auto-detection of
    /// JSON vs compact format, use [`verify_auto`](Self::verify_auto).
    pub fn verify(&self, license_data: &str) -> Result<LicensePayload, LicenseError> {
        // Parse the license file JSON
        let license_file: LicenseFile = serde_json::from_str(license_data)?;

        // Decode payload and signature
        let payload_bytes = BASE64_STANDARD.decode(&license_file.payload)?;
        let sig_bytes = BASE64_STANDARD.decode(&license_file.sig)?;

        // Convert signature bytes
        let sig_bytes: [u8; 64] = sig_bytes
            .try_into()
            .map_err(|_| LicenseError::InvalidFormat("signature must be 64 bytes".into()))?;
        let signature = Signature::from_bytes(&sig_bytes);

        // Verify signature against payload
        self.public_key
            .verify(&payload_bytes, &signature)
            .map_err(|_| LicenseError::InvalidSignature)?;

        // Signature valid - decode payload
        let payload: LicensePayload = serde_json::from_slice(&payload_bytes)?;

        Ok(payload)
    }

    /// Verify signature, decode payload, and reject expired licenses.
    ///
    /// This is the recommended method for most use cases. Returns
    /// [`LicenseError::Expired`] if `exp` is in the past. Perpetual
    /// licenses (`exp: None`) always pass the expiry check.
    pub fn verify_active(&self, license_data: &str) -> Result<LicensePayload, LicenseError> {
        let payload = self.verify(license_data)?;
        if let Some(exp) = payload.exp {
            if payload.is_expired() {
                return Err(LicenseError::Expired(exp));
            }
        }
        Ok(payload)
    }

    /// Verify signature, check expiry, and require a specific entitlement.
    ///
    /// Returns [`LicenseError::MissingEntitlement`] if the entitlement
    /// is not in the payload's `ent` list. Matching is exact and
    /// case-sensitive.
    pub fn verify_with_entitlement(
        &self,
        license_data: &str,
        required: &str,
    ) -> Result<LicensePayload, LicenseError> {
        let payload = self.verify_active(license_data)?;
        if !payload.has_entitlement(required) {
            return Err(LicenseError::MissingEntitlement(required.to_string()));
        }
        Ok(payload)
    }

    /// Verify a license in either JSON or compact format.
    ///
    /// Auto-detects the format: strings starting with `{` are parsed as
    /// JSON, everything else as compact (`payload.signature`). Use this
    /// when loading licenses from files or user input where the format
    /// is not known in advance.
    ///
    /// Only checks the signature — combine with
    /// [`LicensePayload::is_expired`] or
    /// [`LicensePayload::has_entitlement`] for additional checks.
    pub fn verify_auto(&self, license_data: &str) -> Result<LicensePayload, LicenseError> {
        let trimmed = license_data.trim();

        // Try compact format first (no braces)
        if !trimmed.starts_with('{') {
            let license_file = LicenseFile::from_compact(trimmed)
                .map_err(|e| LicenseError::InvalidFormat(e.to_string()))?;
            let json = serde_json::to_string(&license_file)?;
            return self.verify(&json);
        }

        // Otherwise parse as JSON
        self.verify(trimmed)
    }
}
