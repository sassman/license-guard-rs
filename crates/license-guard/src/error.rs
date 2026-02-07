use thiserror::Error;

#[derive(Error, Debug)]
pub enum LicenseError {
    #[error("invalid license format: {0}")]
    InvalidFormat(String),

    #[error("invalid signature")]
    InvalidSignature,

    #[error("license expired")]
    Expired,

    #[error("missing entitlement: {0}")]
    MissingEntitlement(String),

    #[error("invalid public key: {0}")]
    InvalidPublicKey(String),

    #[error("base64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),

    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),
}
