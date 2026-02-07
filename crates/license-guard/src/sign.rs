use base64::prelude::*;
use ed25519_dalek::Signer;

use crate::error::LicenseError;
use crate::license::{LicenseFile, LicensePayload};

/// Re-export `SigningKey` so consumers don't need to depend on `ed25519-dalek` directly.
pub use ed25519_dalek::SigningKey;

impl LicensePayload {
    /// Sign this payload with the given Ed25519 signing key.
    ///
    /// Returns a [`LicenseFile`] containing the base64-encoded payload and signature.
    /// The resulting license can be serialized to JSON, compact format, or base64
    /// for distribution.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use license_guard::{LicensePayload, sign::SigningKey};
    ///
    /// let payload = LicensePayload {
    ///     v: 1,
    ///     sub: "user@example.com".into(),
    ///     iss: "my-app".into(),
    ///     iat: 1706400000,
    ///     exp: Some(1737936000),
    ///     ent: vec!["premium".into()],
    ///     meta: Default::default(),
    /// };
    ///
    /// let license_file = payload.sign(&signing_key)?;
    /// let json = serde_json::to_string(&license_file)?;
    /// ```
    pub fn sign(&self, signing_key: &SigningKey) -> Result<LicenseFile, LicenseError> {
        let payload_json = serde_json::to_string(self)?;
        let signature = signing_key.sign(payload_json.as_bytes());

        Ok(LicenseFile {
            payload: BASE64_STANDARD.encode(payload_json.as_bytes()),
            sig: BASE64_STANDARD.encode(signature.to_bytes()),
        })
    }
}
