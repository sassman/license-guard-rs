//! Global license state for easy access from anywhere in the application.
//!
//! This module provides a singleton pattern so you don't need to pass
//! the license verifier through your entire call chain.
//!
//! # Usage
//!
//! ```rust,ignore
//! use license_guard::global;
//!
//! // 1. Initialize once at app startup
//! global::init("your_public_key_hex")?;
//!
//! // 2. Activate when user enters license
//! global::activate(license_data)?;
//!
//! // 3. Check entitlements from anywhere
//! if global::has("feature_name") {
//!     // feature is enabled
//! }
//! ```

use std::sync::{OnceLock, RwLock};
use crate::{LicenseError, LicensePayload, LicenseVerifier};

static VERIFIER: OnceLock<LicenseVerifier> = OnceLock::new();
static LICENSE: RwLock<Option<LicensePayload>> = RwLock::new(None);

/// Initialize the license system with your public key.
///
/// Call this once at application startup before any other license operations.
///
/// # Errors
///
/// Returns error if the public key is invalid or if already initialized.
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

/// Activate a license. Can be called multiple times (e.g., when user enters new license).
///
/// # Errors
///
/// Returns error if license system not initialized, or if license is invalid/expired.
pub fn activate(license_data: &str) -> Result<LicensePayload, LicenseError> {
    let verifier = VERIFIER.get()
        .ok_or_else(|| LicenseError::InvalidFormat("license system not initialized - call init() first".into()))?;
    let payload = verifier.verify_active(license_data)?;
    *LICENSE.write().unwrap() = Some(payload.clone());
    Ok(payload)
}

/// Clear the current license (e.g., for logout or reset).
pub fn deactivate() {
    *LICENSE.write().unwrap() = None;
}

/// Check if a specific entitlement is present in the current license.
///
/// Returns `false` if no license is active or if the entitlement is not present.
pub fn has(entitlement: &str) -> bool {
    LICENSE.read().unwrap()
        .as_ref()
        .map(|p| p.has_entitlement(entitlement))
        .unwrap_or(false)
}

/// Check if any valid license is currently active.
pub fn is_licensed() -> bool {
    LICENSE.read().unwrap().is_some()
}

/// Get the licensee identifier (e.g., email) if a license is active.
pub fn licensee() -> Option<String> {
    LICENSE.read().unwrap()
        .as_ref()
        .map(|p| p.sub.clone())
}

/// Get the full license payload if a license is active.
pub fn payload() -> Option<LicensePayload> {
    LICENSE.read().unwrap().clone()
}

#[cfg(test)]
mod tests {
    // Note: These tests must run serially due to global state
    // Use `cargo test -- --test-threads=1` or separate test binaries
}
