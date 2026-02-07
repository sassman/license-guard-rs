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
//! For multiple products or testing, use [`LicenseVerifier`] directly.

use std::sync::{OnceLock, RwLock};
use crate::{LicenseError, LicensePayload, LicenseVerifier};

static VERIFIER: OnceLock<LicenseVerifier> = OnceLock::new();
static LICENSE: RwLock<Option<LicensePayload>> = RwLock::new(None);

/// Initialize with your public key. Call once at startup.
pub fn init(public_key_hex: &str) -> Result<(), LicenseError> {
    let verifier = LicenseVerifier::from_hex(public_key_hex)?;
    VERIFIER.set(verifier).map_err(|_|
        LicenseError::InvalidFormat("license system already initialized".into()))?;
    Ok(())
}

/// Check if the license system has been initialized.
pub fn is_initialized() -> bool {
    VERIFIER.get().is_some()
}

/// Activate a license. Can be called multiple times.
pub fn activate(license_data: &str) -> Result<LicensePayload, LicenseError> {
    let verifier = VERIFIER.get()
        .ok_or_else(|| LicenseError::InvalidFormat("license system not initialized - call init() first".into()))?;
    let payload = verifier.verify_active(license_data)?;
    *LICENSE.write().unwrap() = Some(payload.clone());
    Ok(payload)
}

/// Clear current license.
pub fn deactivate() {
    *LICENSE.write().unwrap() = None;
}

/// Check if an entitlement is present.
pub fn has(entitlement: &str) -> bool {
    LICENSE.read().unwrap()
        .as_ref()
        .map(|p| p.has_entitlement(entitlement))
        .unwrap_or(false)
}

/// Check if any license is active.
pub fn is_licensed() -> bool {
    LICENSE.read().unwrap().is_some()
}

/// Get licensee identifier if licensed.
pub fn licensee() -> Option<String> {
    LICENSE.read().unwrap()
        .as_ref()
        .map(|p| p.sub.clone())
}

/// Get full license payload if licensed.
pub fn payload() -> Option<LicensePayload> {
    LICENSE.read().unwrap().clone()
}

#[cfg(test)]
mod tests {
    // Note: These tests must run serially due to global state
    // Use `cargo test -- --test-threads=1` or separate test binaries
}
