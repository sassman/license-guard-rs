use thiserror::Error;

/// Errors returned by license parsing, verification, and validation.
///
/// Variants are ordered roughly by where they occur in the pipeline:
///
/// 1. **Parsing** — [`InvalidFormat`](Self::InvalidFormat),
///    [`Base64Error`](Self::Base64Error), [`JsonError`](Self::JsonError)
/// 2. **Key loading** — [`InvalidPublicKey`](Self::InvalidPublicKey)
/// 3. **Cryptographic verification** — [`InvalidSignature`](Self::InvalidSignature)
/// 4. **Business rules** — [`Expired`](Self::Expired),
///    [`MissingEntitlement`](Self::MissingEntitlement)
///
/// # Example
///
/// ```rust,ignore
/// use license_guard::LicenseError;
///
/// match verifier.verify_active(data) {
///     Ok(payload) => println!("Licensed to {}", payload.sub),
///     Err(LicenseError::InvalidSignature) => eprintln!("Invalid or tampered license"),
///     Err(LicenseError::Expired(ts)) => eprintln!("License expired at {ts}"),
///     Err(LicenseError::MissingEntitlement(e)) => eprintln!("Need entitlement: {e}"),
///     Err(e) => eprintln!("License error: {e}"),
/// }
/// ```
#[derive(Error, Debug)]
pub enum LicenseError {
    /// License data is malformed — bad JSON structure, wrong compact
    /// format, unreadable file, or invalid base64 content.
    #[error("invalid license format: {0}")]
    InvalidFormat(String),

    /// Ed25519 signature does not match the payload. The license was
    /// either tampered with or signed by a different key.
    #[error("invalid signature")]
    InvalidSignature,

    /// License `exp` timestamp is in the past. The `u64` value is the
    /// expiry timestamp (Unix seconds, UTC).
    #[error("license expired at {0}")]
    Expired(u64),

    /// A required entitlement is not present in the license's `ent` list.
    #[error("missing entitlement: {0}")]
    MissingEntitlement(String),

    /// The public key could not be decoded — wrong hex, wrong length
    /// (must be 32 bytes), or not a valid Ed25519 point.
    #[error("invalid public key: {0}")]
    InvalidPublicKey(String),

    /// Base64 decoding failed while parsing the license payload or signature.
    #[error("base64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),

    /// JSON parsing failed while reading the license file or payload.
    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),
}
