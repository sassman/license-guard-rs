use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// License payload - the signed data.
///
/// Decode with [`LicenseVerifier::verify`](crate::LicenseVerifier::verify)
/// or [`global::activate`](crate::global::activate).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensePayload {
    /// Schema version (currently 1)
    pub v: u32,
    /// Licensee identifier (email, user ID, etc.)
    pub sub: String,
    /// Product identifier
    pub iss: String,
    /// Issue timestamp (Unix seconds)
    pub iat: u64,
    /// Expiry timestamp (Unix seconds), None = never expires
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<u64>,
    /// Enabled features
    #[serde(default)]
    pub ent: Vec<String>,
    /// Custom key-value data
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub meta: HashMap<String, String>,
}

/// License file format with payload and signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseFile {
    /// Base64-encoded JSON payload
    pub payload: String,
    /// Base64-encoded Ed25519 signature
    pub sig: String,
}

impl LicenseFile {
    /// Convert to compact format: `payload.signature`
    pub fn to_compact(&self) -> String {
        format!("{}.{}", self.payload, self.sig)
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

    /// Check if license is expired
    pub fn is_expired(&self) -> bool {
        self.exp.map(|exp| Self::now_secs() > exp).unwrap_or(false)
    }

    /// Time remaining until expiration.
    /// Returns `None` if perpetual or already expired.
    pub fn expires_in(&self) -> Option<std::time::Duration> {
        let exp = self.exp?;
        let now = Self::now_secs();
        if now >= exp {
            None
        } else {
            Some(std::time::Duration::from_secs(exp - now))
        }
    }

    /// Time since expiration.
    /// Returns `None` if perpetual or not yet expired.
    pub fn expired_since(&self) -> Option<std::time::Duration> {
        let exp = self.exp?;
        let now = Self::now_secs();
        if now <= exp {
            None
        } else {
            Some(std::time::Duration::from_secs(now - exp))
        }
    }

    /// Check if license has a specific entitlement
    pub fn has_entitlement(&self, entitlement: &str) -> bool {
        self.ent.iter().any(|e| e == entitlement)
    }
}
