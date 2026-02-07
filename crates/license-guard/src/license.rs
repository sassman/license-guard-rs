use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The signed license payload containing licensee info, entitlements, and expiry.
///
/// This is the data that gets signed by Ed25519. After verification with
/// [`LicenseVerifier::verify`](crate::LicenseVerifier::verify) or
/// [`global::activate`](crate::global::activate), you get a `LicensePayload`
/// to inspect.
///
/// Field names follow JWT-style conventions (`sub`, `iss`, `iat`, `exp`, `ent`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensePayload {
    /// Schema version. Currently `1`. Future versions may add fields;
    /// unknown fields are silently ignored by serde, so forward
    /// compatibility is preserved.
    pub v: u32,
    /// Licensee identifier — typically an email address or user ID.
    /// Use this to display "Licensed to …" in your UI.
    pub sub: String,
    /// Product identifier — the product this license was issued for
    /// (e.g. `"my-app"`). Matches the product name in `license-forge`.
    pub iss: String,
    /// Issue timestamp as Unix seconds (UTC).
    pub iat: u64,
    /// Expiry timestamp as Unix seconds (UTC). `None` means the license
    /// never expires (perpetual). Check with [`is_expired`](Self::is_expired)
    /// or let [`LicenseVerifier::verify_active`](crate::LicenseVerifier::verify_active)
    /// do it for you.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<u64>,
    /// Entitlements — feature flags the licensee has access to
    /// (e.g. `["premium", "f5"]`). Check with
    /// [`has_entitlement`](Self::has_entitlement) or [`global::has`](crate::global::has).
    #[serde(default)]
    pub ent: Vec<String>,
    /// Arbitrary key-value metadata. Use for anything that doesn't fit the
    /// standard fields — e.g. `seats`, `org`, `tier`. Omitted from
    /// serialization when empty.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub meta: HashMap<String, String>,
}

/// A signed license file containing a base64-encoded payload and its Ed25519 signature.
///
/// Two serialization formats are supported:
///
/// - **JSON** — `{"payload":"<base64>","sig":"<base64>"}`
/// - **Compact** — `<base64-payload>.<base64-sig>` (one line, dot-separated)
///
/// Use [`from_path`](Self::from_path) to load from disk,
/// [`from_base64`](Self::from_base64) to decode a pasted license key, or
/// [`from_compact`](Self::from_compact) to parse the dot-separated format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseFile {
    /// Base64-encoded JSON payload (the serialized [`LicensePayload`])
    pub payload: String,
    /// Base64-encoded Ed25519 signature over the raw JSON payload bytes
    pub sig: String,
}

impl LicenseFile {
    /// Convert to compact format: `payload.signature`
    pub fn to_compact(&self) -> String {
        format!("{}.{}", self.payload, self.sig)
    }

    /// Read and parse a license from a file path.
    ///
    /// Supports both JSON and compact (`payload.signature`) formats.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use license_guard::LicenseFile;
    ///
    /// let license = LicenseFile::from_path("license.json")?;
    /// // or compact format:
    /// let license = LicenseFile::from_path("license.lic")?;
    /// ```
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self, crate::LicenseError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::LicenseError::InvalidFormat(e.to_string()))?;
        let trimmed = content.trim();
        if trimmed.starts_with('{') {
            Ok(serde_json::from_str(trimmed)?)
        } else {
            Self::from_compact(trimmed)
                .map_err(|e| crate::LicenseError::InvalidFormat(e.to_string()))
        }
    }

    /// Decode a license from a base64-encoded string.
    ///
    /// Use this when the user pastes a license key into a text field.
    /// The base64 content may be JSON or compact format underneath.
    /// Whitespace around the input is trimmed.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use license_guard::{LicenseFile, LicenseVerifier};
    ///
    /// let user_input = "eyJwYXlsb2FkIjoi..."; // pasted from email/UI
    /// let license = LicenseFile::from_base64(user_input)?;
    ///
    /// // validate before persisting
    /// let verifier = LicenseVerifier::from_hex(public_key)?;
    /// let payload = verifier.verify_active(&serde_json::to_string(&license)?)?;
    ///
    /// // now safe to save
    /// std::fs::write("license.json", serde_json::to_string(&license)?)?;
    /// ```
    pub fn from_base64(encoded: &str) -> Result<Self, crate::LicenseError> {
        use base64::prelude::*;
        let decoded = BASE64_STANDARD
            .decode(encoded.trim().as_bytes())
            .map_err(|e| crate::LicenseError::InvalidFormat(e.to_string()))?;
        let text = String::from_utf8(decoded)
            .map_err(|e| crate::LicenseError::InvalidFormat(e.to_string()))?;
        let trimmed = text.trim();
        if trimmed.starts_with('{') {
            Ok(serde_json::from_str(trimmed)?)
        } else {
            Self::from_compact(trimmed)
                .map_err(|e| crate::LicenseError::InvalidFormat(e.to_string()))
        }
    }

    /// Parse from compact format: `payload.signature`
    pub fn from_compact(s: &str) -> Result<Self, &'static str> {
        let parts: Vec<&str> = s.trim().split('.').collect();
        if parts.len() != 2 {
            return Err("invalid compact format: expected payload.signature");
        }
        Ok(Self {
            payload: parts[0].to_string(),
            sig: parts[1].to_string(),
        })
    }
}

impl LicensePayload {
    fn now_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Check if the license has expired.
    ///
    /// Returns `false` for perpetual licenses (`exp: None`).
    /// Compares against the current system time (UTC).
    pub fn is_expired(&self) -> bool {
        self.exp.map(|exp| Self::now_secs() > exp).unwrap_or(false)
    }

    /// Time remaining until expiration.
    ///
    /// Returns `None` if the license is perpetual (`exp: None`) or already expired.
    pub fn expires_in(&self) -> Option<std::time::Duration> {
        let exp = self.exp?;
        let now = Self::now_secs();
        if now >= exp {
            None
        } else {
            Some(std::time::Duration::from_secs(exp - now))
        }
    }

    /// Time elapsed since expiration.
    ///
    /// Returns `None` if the license is perpetual (`exp: None`) or still valid.
    pub fn expired_since(&self) -> Option<std::time::Duration> {
        let exp = self.exp?;
        let now = Self::now_secs();
        if now <= exp {
            None
        } else {
            Some(std::time::Duration::from_secs(now - exp))
        }
    }

    /// Check if the license includes a specific entitlement.
    ///
    /// Entitlements are arbitrary strings defined by your application
    /// (e.g. `"premium"`, `"export"`, `"f5"`). Matching is exact and
    /// case-sensitive.
    pub fn has_entitlement(&self, entitlement: &str) -> bool {
        self.ent.iter().any(|e| e == entitlement)
    }
}
