use thiserror::Error;

/// License validation errors.
#[derive(Error, Debug)]
pub enum LicenseError {
    /// License string is malformed (bad JSON, wrong structure)
    #[error("invalid license format: {0}")]
    InvalidFormat(String),

    /// Signature doesn't match payload (tampered or wrong key)
    #[error("invalid signature")]
    InvalidSignature,

    /// License `exp` timestamp is in the past
    #[error("license expired")]
    Expired,

    /// Required entitlement not in `ent` list
    #[error("missing entitlement: {0}")]
    MissingEntitlement(String),

    /// Public key is malformed or wrong length
    #[error("invalid public key: {0}")]
    InvalidPublicKey(String),

    #[error("base64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),

    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),
}
