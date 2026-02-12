//! Offline license validation using Ed25519 signatures.
//!
//! `license-guard` lets you gate features behind cryptographically signed
//! licenses — no server, no phone-home, no runtime dependency beyond the
//! binary itself. Licenses are generated with the
//! [`license-forge`](https://crates.io/crates/license-forge) CLI.
//!
//! # Getting Started
//!
//! **1. Generate a keypair** (one-time, with `license-forge`):
//!
//! ```bash,ignore
//! license-forge init my-product
//! ```
//!
//! **2. Embed the public key** and check licenses at runtime:
//!
//! ```rust,ignore
//! use license_guard::global;
//!
//! // Initialize once at startup with your public key
//! global::init("your_public_key_hex")?;
//!
//! // Activate when the user enters a license
//! global::activate(license_data)?;
//!
//! // Check entitlements anywhere — no context passing needed
//! if global::has("premium") {
//!     // unlock premium feature
//! }
//! ```
//!
//! **3. Issue licenses** to users (with `license-forge`):
//!
//! ```bash,ignore
//! license-forge generate --sub "user@example.com" --ent premium
//! ```
//!
//! # Use Cases
//!
//! - **Desktop app with feature gating** — embed the public key, call
//!   [`global::init`] at startup, and sprinkle [`global::has`] checks
//!   wherever you need to gate features.
//! - **CLI tool with premium commands** — check entitlements before
//!   executing paid sub-commands; free commands work without a license.
//! - **Library with tiered API access** — use [`LicenseVerifier`] directly
//!   so callers can supply their own keys and license data.
//!
//! # Choosing an API
//!
//! | Scenario | API | Why |
//! |----------|-----|-----|
//! | Most apps (single product) | [`global`] module | No context passing, init-once |
//! | Multiple products / tenants | [`LicenseVerifier`] | One instance per product |
//! | Unit tests | [`LicenseVerifier`] or [`global::LicenseState`] | No global state |
//!
//! # Loading Licenses
//!
//! | Source | Method |
//! |--------|--------|
//! | File on disk | [`LicenseFile::from_path`] — auto-detects JSON / compact |
//! | User text input (base64) | [`LicenseFile::from_base64`] — for paste-in license keys |
//! | Raw JSON or compact string | [`LicenseVerifier::verify_auto`] — auto-detects format |
//!
//! # Error Handling
//!
//! All fallible operations return [`LicenseError`]. Match on variants to
//! give users actionable feedback:
//!
//! ```rust,ignore
//! use license_guard::LicenseError;
//!
//! match verifier.verify_active(license_data) {
//!     Ok(payload) => println!("Licensed to {}", payload.sub),
//!     Err(LicenseError::InvalidSignature) => eprintln!("License is invalid"),
//!     Err(LicenseError::Expired(_)) => eprintln!("License has expired"),
//!     Err(LicenseError::MissingEntitlement(e)) => eprintln!("Missing: {e}"),
//!     Err(e) => eprintln!("License error: {e}"),
//! }
//! ```
//!
//! # Generating Licenses
//!
//! Use the [`license-forge`](https://crates.io/crates/license-forge) CLI
//! to generate Ed25519 keypairs and sign licenses. See the
//! [integration example](https://github.com/sassman/license-guard-rs/blob/main/crates/license-guard/examples/integration.rs)
//! for a full walkthrough.

mod error;
pub mod global;
mod license;
#[cfg(any(test, feature = "sign"))]
pub mod sign;
mod verify;

pub use error::LicenseError;
pub use license::{LicenseFile, LicensePayload};
pub use verify::LicenseVerifier;

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn create_test_keypair() -> (SigningKey, String) {
        let signing_key = SigningKey::generate(&mut rand::rng());
        let public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());
        (signing_key, public_key_hex)
    }

    fn sign_payload(signing_key: &SigningKey, payload: &LicensePayload) -> LicenseFile {
        payload.sign(signing_key).unwrap()
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

    #[test]
    fn test_from_path_json() {
        let (signing_key, public_key_hex) = create_test_keypair();
        let verifier = LicenseVerifier::from_hex(&public_key_hex).unwrap();

        let payload = LicensePayload {
            v: 1,
            sub: "path@test.com".into(),
            iss: "test-app".into(),
            iat: 1706400000,
            exp: None,
            ent: vec![],
            meta: Default::default(),
        };

        let license_file = sign_payload(&signing_key, &payload);
        let json = serde_json::to_string(&license_file).unwrap();

        let tmp = std::env::temp_dir().join("license-guard-test.json");
        std::fs::write(&tmp, &json).unwrap();

        let loaded = LicenseFile::from_path(&tmp).unwrap();
        let license_json = serde_json::to_string(&loaded).unwrap();
        let result = verifier.verify(&license_json).unwrap();
        assert_eq!(result.sub, "path@test.com");

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_from_path_compact() {
        let (signing_key, public_key_hex) = create_test_keypair();
        let verifier = LicenseVerifier::from_hex(&public_key_hex).unwrap();

        let payload = LicensePayload {
            v: 1,
            sub: "compact@test.com".into(),
            iss: "test-app".into(),
            iat: 1706400000,
            exp: None,
            ent: vec![],
            meta: Default::default(),
        };

        let license_file = sign_payload(&signing_key, &payload);
        let compact = license_file.to_compact();

        let tmp = std::env::temp_dir().join("license-guard-test.lic");
        std::fs::write(&tmp, &compact).unwrap();

        let loaded = LicenseFile::from_path(&tmp).unwrap();
        let license_json = serde_json::to_string(&loaded).unwrap();
        let result = verifier.verify(&license_json).unwrap();
        assert_eq!(result.sub, "compact@test.com");

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_from_base64_json() {
        use base64::prelude::*;

        let (signing_key, public_key_hex) = create_test_keypair();
        let verifier = LicenseVerifier::from_hex(&public_key_hex).unwrap();

        let payload = LicensePayload {
            v: 1,
            sub: "b64@test.com".into(),
            iss: "test-app".into(),
            iat: 1706400000,
            exp: None,
            ent: vec!["pro".into()],
            meta: Default::default(),
        };

        let license_file = sign_payload(&signing_key, &payload);
        let json = serde_json::to_string(&license_file).unwrap();
        let encoded = BASE64_STANDARD.encode(json.as_bytes());

        let loaded = LicenseFile::from_base64(&encoded).unwrap();
        let license_json = serde_json::to_string(&loaded).unwrap();
        let result = verifier.verify(&license_json).unwrap();
        assert_eq!(result.sub, "b64@test.com");
    }

    #[test]
    fn test_from_base64_compact() {
        use base64::prelude::*;

        let (signing_key, public_key_hex) = create_test_keypair();
        let verifier = LicenseVerifier::from_hex(&public_key_hex).unwrap();

        let payload = LicensePayload {
            v: 1,
            sub: "b64compact@test.com".into(),
            iss: "test-app".into(),
            iat: 1706400000,
            exp: None,
            ent: vec![],
            meta: Default::default(),
        };

        let license_file = sign_payload(&signing_key, &payload);
        let compact = license_file.to_compact();
        let encoded = BASE64_STANDARD.encode(compact.as_bytes());

        let loaded = LicenseFile::from_base64(&encoded).unwrap();
        let license_json = serde_json::to_string(&loaded).unwrap();
        let result = verifier.verify(&license_json).unwrap();
        assert_eq!(result.sub, "b64compact@test.com");
    }

    #[test]
    fn test_from_base64_with_whitespace() {
        use base64::prelude::*;

        let (signing_key, public_key_hex) = create_test_keypair();
        let verifier = LicenseVerifier::from_hex(&public_key_hex).unwrap();

        let payload = LicensePayload {
            v: 1,
            sub: "ws@test.com".into(),
            iss: "test-app".into(),
            iat: 1706400000,
            exp: None,
            ent: vec![],
            meta: Default::default(),
        };

        let license_file = sign_payload(&signing_key, &payload);
        let json = serde_json::to_string(&license_file).unwrap();
        let encoded = format!("  {} \n", BASE64_STANDARD.encode(json.as_bytes()));

        let loaded = LicenseFile::from_base64(&encoded).unwrap();
        let license_json = serde_json::to_string(&loaded).unwrap();
        let result = verifier.verify(&license_json).unwrap();
        assert_eq!(result.sub, "ws@test.com");
    }

    #[test]
    fn test_from_base64_invalid() {
        let result = LicenseFile::from_base64("not-valid-base64!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_sign_and_verify_roundtrip() {
        let (signing_key, public_key_hex) = create_test_keypair();
        let verifier = LicenseVerifier::from_hex(&public_key_hex).unwrap();

        let payload = LicensePayload {
            v: 1,
            sub: "sign-test@example.com".into(),
            iss: "test-app".into(),
            iat: 1706400000,
            exp: None,
            ent: vec!["premium".into()],
            meta: Default::default(),
        };

        // Use the new sign() method
        let license_file = payload.sign(&signing_key).unwrap();

        // Verify it works with the verifier
        let license_json = serde_json::to_string(&license_file).unwrap();
        let result = verifier.verify(&license_json).unwrap();
        assert_eq!(result.sub, "sign-test@example.com");
        assert_eq!(result.ent, vec!["premium"]);
    }
}
