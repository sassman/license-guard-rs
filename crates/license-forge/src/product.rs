use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Product configuration stored in product.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    /// Human-readable product name/identifier
    pub name: String,
    /// Available entitlements for this product
    pub entitlements: Vec<String>,
}

impl Product {
    const PRIVATE_KEY_FILE: &'static str = "license.private";
    const PUBLIC_KEY_FILE: &'static str = "license.pub";

    /// Base directory for all products: ~/.config/license-forge/
    pub fn base_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("license-forge")
    }

    /// Directory for a specific product: ~/.config/license-forge/<product>/
    pub fn dir(product_name: &str) -> PathBuf {
        Self::base_dir().join(product_name)
    }

    /// Path to product config file
    pub fn config_path(product_name: &str) -> PathBuf {
        Self::dir(product_name).join("product.toml")
    }

    /// Path to private key
    pub fn private_key_path(product_name: &str) -> PathBuf {
        Self::dir(product_name).join(Self::PRIVATE_KEY_FILE)
    }

    /// Path to public key
    pub fn public_key_path(product_name: &str) -> PathBuf {
        Self::dir(product_name).join(Self::PUBLIC_KEY_FILE)
    }

    /// Directory for licenses
    pub fn licenses_dir(product_name: &str) -> PathBuf {
        Self::dir(product_name).join("licenses")
    }

    /// Check if a product exists
    pub fn exists(product_name: &str) -> bool {
        Self::config_path(product_name).exists()
    }

    /// List all product names
    pub fn list() -> Vec<String> {
        let base = Self::base_dir();
        if !base.exists() {
            return vec![];
        }
        fs::read_dir(base)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .filter(|e| e.path().join("product.toml").exists())
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Load product from disk
    pub fn load(product_name: &str) -> anyhow::Result<Self> {
        let path = Self::config_path(product_name);
        let content = fs::read_to_string(&path)?;
        Ok(toml::from_str(&content)?)
    }

    /// Save product to disk (creates directories if needed)
    pub fn save(&self, product_name: &str) -> anyhow::Result<()> {
        let dir = Self::dir(product_name);
        fs::create_dir_all(&dir)?;
        fs::create_dir_all(Self::licenses_dir(product_name))?;

        let content = toml::to_string_pretty(self)?;
        fs::write(Self::config_path(product_name), content)?;
        Ok(())
    }

    /// Remove a product directory entirely
    pub fn remove(product_name: &str) -> anyhow::Result<()> {
        let dir = Self::dir(product_name);
        if dir.exists() {
            fs::remove_dir_all(dir)?;
        }
        Ok(())
    }

    /// Format path with ~ for display
    pub fn display_path(path: &std::path::Path) -> String {
        if let Some(home) = dirs::home_dir() {
            if let Ok(suffix) = path.strip_prefix(&home) {
                return format!("~/{}", suffix.display());
            }
        }
        path.display().to_string()
    }

    /// Generate and save a new keypair for a product
    /// Returns (private_key_hex, public_key_hex)
    pub fn generate_keys(product_name: &str) -> anyhow::Result<(String, String)> {
        let signing_key = SigningKey::generate(&mut rand::rng());
        let verifying_key = signing_key.verifying_key();

        let priv_hex = hex::encode(signing_key.to_bytes());
        let pub_hex = hex::encode(verifying_key.to_bytes());

        let priv_path = Self::private_key_path(product_name);
        let pub_path = Self::public_key_path(product_name);

        // Ensure directory exists
        if let Some(parent) = priv_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&priv_path, &priv_hex)?;
        fs::write(&pub_path, &pub_hex)?;

        Ok((priv_hex, pub_hex))
    }

    /// Backup existing keys if they exist (returns true if backup was made)
    pub fn backup_keys(product_name: &str) -> anyhow::Result<bool> {
        let priv_path = Self::private_key_path(product_name);
        let pub_path = Self::public_key_path(product_name);

        if !priv_path.exists() {
            return Ok(false);
        }

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let priv_backup = Self::dir(product_name).join(format!("{}.{}.bak", Self::PRIVATE_KEY_FILE, timestamp));
        let pub_backup = Self::dir(product_name).join(format!("{}.{}.bak", Self::PUBLIC_KEY_FILE, timestamp));

        fs::copy(&priv_path, &priv_backup)?;
        if pub_path.exists() {
            fs::copy(&pub_path, &pub_backup)?;
        }

        Ok(true)
    }

    /// Load the signing key for a product
    pub fn load_signing_key(product_name: &str) -> anyhow::Result<SigningKey> {
        let priv_path = Self::private_key_path(product_name);
        let priv_hex = fs::read_to_string(&priv_path)?;
        let priv_bytes = hex::decode(priv_hex.trim())?;
        let priv_bytes: [u8; 32] = priv_bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("invalid key length"))?;
        Ok(SigningKey::from_bytes(&priv_bytes))
    }

    /// Get public key hex for a product
    pub fn get_public_key_hex(product_name: &str) -> anyhow::Result<String> {
        let pub_path = Self::public_key_path(product_name);
        Ok(fs::read_to_string(&pub_path)?.trim().to_string())
    }

    /// Check if keys exist for a product
    pub fn has_keys(product_name: &str) -> bool {
        Self::private_key_path(product_name).exists()
    }
}
