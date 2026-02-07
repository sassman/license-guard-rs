use crate::error::LicenseError;
use crate::license::{LicenseFile, LicensePayload};
use base64::prelude::*;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

/// License verifier with embedded public key
pub struct LicenseVerifier {
    public_key: VerifyingKey,
}

impl LicenseVerifier {
    /// Create verifier from hex-encoded public key
    pub fn from_hex(hex_key: &str) -> Result<Self, LicenseError> {
        let bytes = hex::decode(hex_key)
            .map_err(|e| LicenseError::InvalidPublicKey(e.to_string()))?;
        Self::from_bytes(&bytes)
    }

    /// Create verifier from raw public key bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, LicenseError> {
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| LicenseError::InvalidPublicKey("key must be 32 bytes".into()))?;
        let public_key = VerifyingKey::from_bytes(&bytes)
            .map_err(|e| LicenseError::InvalidPublicKey(e.to_string()))?;
        Ok(Self { public_key })
    }

    /// Verify and decode a license file
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

    /// Verify, decode, and check expiry
    pub fn verify_active(&self, license_data: &str) -> Result<LicensePayload, LicenseError> {
        let payload = self.verify(license_data)?;
        if payload.is_expired() {
            return Err(LicenseError::Expired);
        }
        Ok(payload)
    }

    /// Verify, decode, check expiry, and require specific entitlement
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
}
