//! Integration test for the global module's static init/activate/deactivate flow.
//! This runs as a separate binary, so the OnceLock does not interfere with other tests.

use ed25519_dalek::{Signer, SigningKey};
use license_guard::{global, LicenseFile, LicensePayload};

fn make_license(sk: &SigningKey, payload: &LicensePayload) -> String {
    use base64::prelude::*;
    let json = serde_json::to_string(payload).unwrap();
    let sig = sk.sign(json.as_bytes());
    let file = LicenseFile {
        payload: BASE64_STANDARD.encode(json.as_bytes()),
        sig: BASE64_STANDARD.encode(sig.to_bytes()),
    };
    serde_json::to_string(&file).unwrap()
}

#[test]
fn global_init_activate_check_deactivate() {
    let sk = SigningKey::generate(&mut rand::thread_rng());
    let pk_hex = hex::encode(sk.verifying_key().to_bytes());

    global::init(&pk_hex).unwrap();
    assert!(global::is_initialized());
    assert!(!global::is_licensed());

    let payload = LicensePayload {
        v: 1,
        sub: "integration@test.com".into(),
        iss: "test-app".into(),
        iat: 1706400000,
        exp: None,
        ent: vec!["pro".into()],
        meta: Default::default(),
    };
    let license_data = make_license(&sk, &payload);

    global::activate(&license_data).unwrap();
    assert!(global::is_licensed());
    assert!(global::has("pro"));
    assert!(!global::has("enterprise"));
    assert_eq!(global::licensee().unwrap(), "integration@test.com");

    global::deactivate();
    assert!(!global::is_licensed());
    assert!(global::licensee().is_none());
}
