//! Offline license validation using Ed25519 signatures.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use license_guard::global;
//!
//! // 1. Initialize once at startup
//! global::init("your_public_key_hex")?;
//!
//! // 2. Activate when user enters license
//! global::activate(license_data)?;
//!
//! // 3. Check entitlements anywhere in your code
//! if global::has("premium") {
//!     // premium feature
//! }
//! ```
//!
//! # Choosing an API
//!
//! | Use case | API |
//! |----------|-----|
//! | Most apps | [`global`] module - no context passing |
//! | Multiple products | [`LicenseVerifier`] - one per product |
//! | Testing | [`LicenseVerifier`] - no global state |
//!
//! # Generating Licenses
//!
//! Use the `license-forge` CLI tool to generate keypairs and sign licenses.
//! See the repository README for details.

mod error;
mod license;
mod verify;
pub mod global;

pub use error::LicenseError;
pub use license::{LicenseFile, LicensePayload};
pub use verify::LicenseVerifier;

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn create_test_keypair() -> (SigningKey, String) {
        let signing_key = SigningKey::generate(&mut rand::rng());
        let public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());
        (signing_key, public_key_hex)
    }

    fn sign_payload(signing_key: &SigningKey, payload: &LicensePayload) -> LicenseFile {
        use base64::prelude::*;
        let payload_json = serde_json::to_string(payload).unwrap();
        let payload_b64 = BASE64_STANDARD.encode(payload_json.as_bytes());
        let signature = signing_key.sign(payload_json.as_bytes());
        let sig_b64 = BASE64_STANDARD.encode(signature.to_bytes());
        LicenseFile {
            payload: payload_b64,
            sig: sig_b64,
        }
    }

    #[test]
    fn test_valid_license() {
        let (signing_key, public_key_hex) = create_test_keypair();
        let verifier = LicenseVerifier::from_hex(&public_key_hex).unwrap();

        let payload = LicensePayload {
            v: 1,
            sub: "test@example.com".into(),
            iss: "test-app".into(),
            iat: 1706400000,
            exp: None,
            ent: vec!["feature1".into()],
            meta: Default::default(),
        };

        let license_file = sign_payload(&signing_key, &payload);
        let license_json = serde_json::to_string(&license_file).unwrap();

        let result = verifier.verify(&license_json).unwrap();
        assert_eq!(result.sub, "test@example.com");
    }

    #[test]
    fn test_tampered_license() {
        let (signing_key, public_key_hex) = create_test_keypair();
        let verifier = LicenseVerifier::from_hex(&public_key_hex).unwrap();

        let payload = LicensePayload {
            v: 1,
            sub: "test@example.com".into(),
            iss: "test-app".into(),
            iat: 1706400000,
            exp: None,
            ent: vec!["feature1".into()],
            meta: Default::default(),
        };

        let mut license_file = sign_payload(&signing_key, &payload);
        // Tamper with payload
        use base64::Engine;
        license_file.payload = base64::prelude::BASE64_STANDARD.encode(b"tampered");
        let license_json = serde_json::to_string(&license_file).unwrap();

        let result = verifier.verify(&license_json);
        assert!(matches!(result, Err(LicenseError::InvalidSignature)));
    }

    #[test]
    fn test_expired_license() {
        let (signing_key, public_key_hex) = create_test_keypair();
        let verifier = LicenseVerifier::from_hex(&public_key_hex).unwrap();

        let payload = LicensePayload {
            v: 1,
            sub: "test@example.com".into(),
            iss: "test-app".into(),
            iat: 1706400000,
            exp: Some(1), // Expired in 1970
            ent: vec![],
            meta: Default::default(),
        };

        let license_file = sign_payload(&signing_key, &payload);
        let license_json = serde_json::to_string(&license_file).unwrap();

        let result = verifier.verify_active(&license_json);
        assert!(matches!(result, Err(LicenseError::Expired(_))));
    }

    #[test]
    fn test_missing_entitlement() {
        let (signing_key, public_key_hex) = create_test_keypair();
        let verifier = LicenseVerifier::from_hex(&public_key_hex).unwrap();

        let payload = LicensePayload {
            v: 1,
            sub: "test@example.com".into(),
            iss: "test-app".into(),
            iat: 1706400000,
            exp: None,
            ent: vec!["feature1".into()],
            meta: Default::default(),
        };

        let license_file = sign_payload(&signing_key, &payload);
        let license_json = serde_json::to_string(&license_file).unwrap();

        let result = verifier.verify_with_entitlement(&license_json, "feature2");
        assert!(matches!(result, Err(LicenseError::MissingEntitlement(_))));
    }
}
