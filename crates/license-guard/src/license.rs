use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// License payload - the data that gets signed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensePayload {
    /// Schema version
    pub v: u32,
    /// Subject (licensee identifier, e.g., email)
    pub sub: String,
    /// Issuer (product identifier)
    pub iss: String,
    /// Issued at (Unix timestamp)
    pub iat: u64,
    /// Expires at (Unix timestamp, None = never expires)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<u64>,
    /// Entitlements (features enabled)
    #[serde(default)]
    pub ent: Vec<String>,
    /// Optional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub meta: HashMap<String, String>,
}

/// Complete license file with payload and signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseFile {
    /// Base64-encoded JSON payload
    pub payload: String,
    /// Base64-encoded Ed25519 signature
    pub sig: String,
}

impl LicenseFile {
    /// Convert to compact single-line format: payload.signature
    pub fn to_compact(&self) -> String {
        format!("{}.{}", self.payload, self.sig)
    }

    /// Parse from compact format (payload.signature)
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
    /// Check if license is expired
    pub fn is_expired(&self) -> bool {
        if let Some(exp) = self.exp {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            now > exp
        } else {
            false
        }
    }

    /// Check if license has a specific entitlement
    pub fn has_entitlement(&self, entitlement: &str) -> bool {
        self.ent.iter().any(|e| e == entitlement)
    }
}
