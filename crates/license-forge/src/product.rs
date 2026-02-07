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
        Self::dir(product_name).join("license.sk")
    }

    /// Path to public key
    pub fn public_key_path(product_name: &str) -> PathBuf {
        Self::dir(product_name).join("license.pk")
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
    pub fn display_path(path: &PathBuf) -> String {
        if let Some(home) = dirs::home_dir() {
            if let Ok(suffix) = path.strip_prefix(&home) {
                return format!("~/{}", suffix.display());
            }
        }
        path.display().to_string()
    }
}
