//! Global license state for apps that validate a single product.
//!
//! # Example
//!
//! ```rust,ignore
//! use license_guard::global;
//!
//! global::init("abc123...")?;
//! global::activate(user_license)?;
//!
//! if global::has("feature") { /* ... */ }
//! ```
//!
//! For multiple products or testing, use [`LicenseVerifier`] directly,
//! or create a [`LicenseState`] instance for testable non-global usage.

use std::sync::{OnceLock, RwLock};
use crate::{LicenseError, LicensePayload, LicenseVerifier};

/// Non-global license state for testable usage and multi-instance scenarios.
///
/// Holds a verifier and an optional active license. Thread-safe via `RwLock`.
///
/// # Example
///
/// ```rust,ignore
/// use license_guard::global::LicenseState;
///
/// let state = LicenseState::new("your_public_key_hex")?;
/// state.activate(license_data)?;
/// assert!(state.has("premium"));
/// ```
pub struct LicenseState {
    verifier: LicenseVerifier,
    license: RwLock<Option<LicensePayload>>,
}

impl LicenseState {
    /// Create a new license state from a hex-encoded public key.
    pub fn new(public_key_hex: &str) -> Result<Self, LicenseError> {
        Ok(Self {
            verifier: LicenseVerifier::from_hex(public_key_hex)?,
            license: RwLock::new(None),
        })
    }

    /// Activate a license. Can be called multiple times.
    pub fn activate(&self, license_data: &str) -> Result<LicensePayload, LicenseError> {
        let payload = self.verifier.verify_active(license_data)?;
        *self.license.write().unwrap() = Some(payload.clone());
        Ok(payload)
    }

    /// Clear current license.
    pub fn deactivate(&self) {
        *self.license.write().unwrap() = None;
    }

    /// Check if an entitlement is present.
    pub fn has(&self, entitlement: &str) -> bool {
        self.license.read().unwrap()
            .as_ref()
            .map(|p| p.has_entitlement(entitlement))
            .unwrap_or(false)
    }

    /// Check if any license is active.
    pub fn is_licensed(&self) -> bool {
        self.license.read().unwrap().is_some()
    }

    /// Get licensee identifier if licensed.
    pub fn licensee(&self) -> Option<String> {
        self.license.read().unwrap()
            .as_ref()
            .map(|p| p.sub.clone())
    }

    /// Get full license payload if licensed.
    pub fn payload(&self) -> Option<LicensePayload> {
        self.license.read().unwrap().clone()
    }
}

// ---------------------------------------------------------------------------
// Global convenience API (delegates to a static LicenseState)
// ---------------------------------------------------------------------------

static STATE: OnceLock<LicenseState> = OnceLock::new();

fn state() -> Result<&'static LicenseState, LicenseError> {
    STATE.get()
        .ok_or_else(|| LicenseError::InvalidFormat(
            "license system not initialized - call init() first".into()
        ))
}

/// Initialize with your public key. Call once at startup.
pub fn init(public_key_hex: &str) -> Result<(), LicenseError> {
    let s = LicenseState::new(public_key_hex)?;
    STATE.set(s).map_err(|_|
        LicenseError::InvalidFormat("license system already initialized".into()))?;
    Ok(())
}

/// Check if the license system has been initialized.
pub fn is_initialized() -> bool {
    STATE.get().is_some()
}

/// Activate a license. Can be called multiple times.
pub fn activate(license_data: &str) -> Result<LicensePayload, LicenseError> {
    state()?.activate(license_data)
}

/// Clear current license.
pub fn deactivate() {
    if let Some(s) = STATE.get() {
        s.deactivate();
    }
}

/// Check if an entitlement is present.
pub fn has(entitlement: &str) -> bool {
    STATE.get().map(|s| s.has(entitlement)).unwrap_or(false)
}

/// Check if any license is active.
pub fn is_licensed() -> bool {
    STATE.get().map(|s| s.is_licensed()).unwrap_or(false)
}

/// Get licensee identifier if licensed.
pub fn licensee() -> Option<String> {
    STATE.get().and_then(|s| s.licensee())
}

/// Get full license payload if licensed.
pub fn payload() -> Option<LicensePayload> {
    STATE.get().and_then(|s| s.payload())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn test_keypair() -> (SigningKey, String) {
        let sk = SigningKey::generate(&mut rand::rng());
        let pk_hex = hex::encode(sk.verifying_key().to_bytes());
        (sk, pk_hex)
    }

    fn sign_license(sk: &SigningKey, payload: &LicensePayload) -> String {
        use base64::prelude::*;
        let json = serde_json::to_string(payload).unwrap();
        let sig = sk.sign(json.as_bytes());
        let file = crate::LicenseFile {
            payload: BASE64_STANDARD.encode(json.as_bytes()),
            sig: BASE64_STANDARD.encode(sig.to_bytes()),
        };
        serde_json::to_string(&file).unwrap()
    }

    fn test_payload(sk: &SigningKey) -> (LicensePayload, String) {
        let payload = LicensePayload {
            v: 1,
            sub: "test@example.com".into(),
            iss: "test-app".into(),
            iat: 1706400000,
            exp: None,
            ent: vec!["feature1".into(), "premium".into()],
            meta: Default::default(),
        };
        let license_data = sign_license(sk, &payload);
        (payload, license_data)
    }

    #[test]
    fn test_license_state_activate_and_check() {
        let (sk, pk_hex) = test_keypair();
        let state = LicenseState::new(&pk_hex).unwrap();
        let (_, license_data) = test_payload(&sk);

        let result = state.activate(&license_data);
        assert!(result.is_ok());
        assert!(state.is_licensed());
        assert!(state.has("feature1"));
        assert!(state.has("premium"));
        assert!(!state.has("nonexistent"));
    }

    #[test]
    fn test_license_state_deactivate() {
        let (sk, pk_hex) = test_keypair();
        let state = LicenseState::new(&pk_hex).unwrap();
        let (_, license_data) = test_payload(&sk);

        state.activate(&license_data).unwrap();
        assert!(state.is_licensed());

        state.deactivate();
        assert!(!state.is_licensed());
        assert!(!state.has("feature1"));
    }

    #[test]
    fn test_license_state_licensee_and_payload() {
        let (sk, pk_hex) = test_keypair();
        let state = LicenseState::new(&pk_hex).unwrap();
        let (_, license_data) = test_payload(&sk);

        assert!(state.licensee().is_none());
        assert!(state.payload().is_none());

        state.activate(&license_data).unwrap();
        assert_eq!(state.licensee().unwrap(), "test@example.com");
        assert!(state.payload().is_some());
    }

    #[test]
    fn test_license_state_not_initialized_without_license() {
        let (_, pk_hex) = test_keypair();
        let state = LicenseState::new(&pk_hex).unwrap();

        assert!(!state.is_licensed());
        assert!(!state.has("anything"));
        assert!(state.licensee().is_none());
    }
}
