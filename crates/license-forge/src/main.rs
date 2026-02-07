mod product;

use base64::prelude::*;
use base64::Engine;
use chrono::{NaiveDate, Utc};
use clap::{Parser, Subcommand};
use dialoguer::{Confirm, Input, MultiSelect, Select};
use ed25519_dalek::{Signer, SigningKey};
use license_guard::{LicenseFile, LicensePayload};
use product::Product;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Product profile for license generation
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Profile {
    /// Product identifier
    product: String,
    /// Available entitlements for this product
    entitlements: Vec<String>,
    /// Path to private key file
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<String>,
}

impl Profile {
    fn load(path: &PathBuf) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    fn save(&self, path: &PathBuf) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    fn profiles_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("license-forge")
            .join("profiles")
    }

    fn list_profiles() -> Vec<String> {
        let dir = Self::profiles_dir();
        if !dir.exists() {
            return vec![];
        }
        fs::read_dir(dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().map(|ext| ext == "toml").unwrap_or(false))
                    .filter_map(|e| e.path().file_stem().map(|s| s.to_string_lossy().to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Parser)]
#[command(name = "license-forge")]
#[command(about = "Generate Ed25519-signed software licenses")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new Ed25519 keypair
    Keygen {
        /// Output directory for keys
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
    },
    /// Generate a new license (interactive)
    Generate {
        /// Path to private key file (can be saved in profile)
        #[arg(short, long)]
        key: Option<PathBuf>,
        /// Load product profile
        #[arg(short, long)]
        profile: Option<String>,
        /// Output file for license
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Verify a license file
    Verify {
        /// Path to public key file
        #[arg(short = 'k', long)]
        public_key: PathBuf,
        /// Path to license file
        #[arg(short, long)]
        license: PathBuf,
    },
    /// Show public key for embedding in app
    ShowPublicKey {
        /// Path to private key file
        #[arg(short, long)]
        key: PathBuf,
    },
    /// List available profiles
    Profiles,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Keygen { output } => keygen(output),
        Commands::Generate { key, profile, output } => generate(key, profile, output),
        Commands::Verify { public_key, license } => verify(public_key, license),
        Commands::ShowPublicKey { key } => show_public_key(key),
        Commands::Profiles => list_profiles(),
    }
}

fn keygen(output: PathBuf) -> anyhow::Result<()> {
    println!("Generating Ed25519 keypair...\n");

    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    // Save private key (hex encoded)
    let sk_path = output.join("license.sk");
    let sk_hex = hex::encode(signing_key.to_bytes());
    fs::write(&sk_path, &sk_hex)?;
    println!("Private key saved to: {}", sk_path.display());
    println!("  KEEP THIS SECRET - never distribute!\n");

    // Save public key (hex encoded)
    let pk_path = output.join("license.pk");
    let pk_hex = hex::encode(verifying_key.to_bytes());
    fs::write(&pk_path, &pk_hex)?;
    println!("Public key saved to: {}", pk_path.display());
    println!("  Embed this in your application\n");

    println!("Public key (hex):\n  {}\n", pk_hex);

    Ok(())
}

fn generate(key_arg: Option<PathBuf>, profile_name: Option<String>, output: Option<PathBuf>) -> anyhow::Result<()> {
    println!("License Generator\n");

    // Load or create profile
    let (mut profile, profile_path) = load_or_create_profile(profile_name)?;

    // Determine key path: CLI argument takes precedence over profile
    let key_path = if let Some(k) = key_arg {
        // CLI provided, update profile with portable path
        profile.key = Some(collapse_home(&k));
        k
    } else if let Some(ref k) = profile.key {
        expand_home(k)
    } else {
        anyhow::bail!("No key provided. Use --key <path> or save a key in the profile.");
    };

    // Load private key
    let sk_hex = fs::read_to_string(&key_path)?;
    let sk_bytes = hex::decode(sk_hex.trim())?;
    let sk_bytes: [u8; 32] = sk_bytes.try_into().map_err(|_| anyhow::anyhow!("invalid key"))?;
    let signing_key = SigningKey::from_bytes(&sk_bytes);

    // Collect license info via dialoguer
    let sub: String = Input::new()
        .with_prompt("Licensee email/identifier")
        .interact_text()?;

    let has_expiry = Confirm::new()
        .with_prompt("Set expiration date?")
        .default(false)
        .interact()?;

    let exp = if has_expiry {
        let date_str: String = Input::new()
            .with_prompt("Expiration date (YYYY-MM-DD)")
            .interact_text()?;
        let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;
        let datetime = date.and_hms_opt(23, 59, 59).unwrap();
        Some(datetime.and_utc().timestamp() as u64)
    } else {
        None
    };

    // Select entitlements from profile, or enter manually if none defined
    let ent = if profile.entitlements.is_empty() {
        println!("Enter entitlements for this license (one per line, empty to finish):");
        let mut entitlements = Vec::new();
        loop {
            let ent: String = Input::new()
                .with_prompt(format!("  Entitlement {}", entitlements.len() + 1))
                .allow_empty(true)
                .interact_text()?;
            if ent.is_empty() {
                break;
            }
            entitlements.push(ent);
        }
        entitlements
    } else {
        let selected = MultiSelect::new()
            .with_prompt("Select entitlements")
            .items(&profile.entitlements)
            .interact()?;
        selected.iter().map(|&i| profile.entitlements[i].clone()).collect()
    };

    // Optional metadata
    let add_meta = Confirm::new()
        .with_prompt("Add custom metadata?")
        .default(false)
        .interact()?;

    let mut meta = HashMap::new();
    if add_meta {
        loop {
            let key: String = Input::new()
                .with_prompt("Metadata key (empty to finish)")
                .allow_empty(true)
                .interact_text()?;
            if key.is_empty() {
                break;
            }
            let value: String = Input::new()
                .with_prompt(&format!("Value for '{}'", key))
                .interact_text()?;
            meta.insert(key, value);
        }
    }

    // Create payload
    let now = Utc::now().timestamp() as u64;
    let payload = LicensePayload {
        v: 1,
        sub: sub.clone(),
        iss: profile.product.clone(),
        iat: now,
        exp,
        ent,
        meta,
    };

    // Sign payload
    let payload_json = serde_json::to_string(&payload)?;
    let signature = signing_key.sign(payload_json.as_bytes());

    let license_file = LicenseFile {
        payload: BASE64_STANDARD.encode(payload_json.as_bytes()),
        sig: BASE64_STANDARD.encode(signature.to_bytes()),
    };

    let license_json = serde_json::to_string_pretty(&license_file)?;

    // Output
    if let Some(output_path) = output {
        fs::write(&output_path, &license_json)?;
        println!("\nLicense saved to: {}", output_path.display());
    } else {
        let default_name = format!("{}.lic", sub.replace('@', "_at_").replace('.', "_"));
        let output_path: String = Input::new()
            .with_prompt("Output filename")
            .default(default_name)
            .interact_text()?;
        fs::write(&output_path, &license_json)?;
        println!("\nLicense saved to: {}", output_path);
    }

    println!("\n--- License Preview ---");
    println!("{}", serde_json::to_string_pretty(&payload)?);

    // Offer to save profile if it was created interactively
    if let Some(path) = profile_path {
        if !path.exists() {
            let save = Confirm::new()
                .with_prompt(format!("Save profile to {}?", display_path(&path)))
                .default(true)
                .interact()?;
            if save {
                profile.save(&path)?;
                println!("\nProfile saved to: {}", display_path(&path));
                println!("Next time run: license-forge generate --profile {}", profile.product);
            }
        }
    }

    Ok(())
}

fn load_or_create_profile(profile_name: Option<String>) -> anyhow::Result<(Profile, Option<PathBuf>)> {
    let existing_profiles = Profile::list_profiles();

    // If profile name provided, try to load it
    if let Some(name) = profile_name {
        let path = Profile::profiles_dir().join(format!("{}.toml", name));
        if path.exists() {
            let profile = Profile::load(&path)?;
            println!("Loaded profile: {} ({})", name, profile.product);
            if let Some(ref key) = profile.key {
                println!("  Key: {}", key);
            }
            println!("  Entitlements: {:?}\n", profile.entitlements);
            return Ok((profile, Some(path)));
        } else {
            anyhow::bail!("Profile '{}' not found at {}", name, path.display());
        }
    }

    // Offer to load existing profile or create new
    if !existing_profiles.is_empty() {
        let mut options: Vec<&str> = existing_profiles.iter().map(|s| s.as_str()).collect();
        options.push("Create new profile");

        let selection = Select::new()
            .with_prompt("Select profile")
            .items(&options)
            .default(0)
            .interact()?;

        if selection < existing_profiles.len() {
            let name = &existing_profiles[selection];
            let path = Profile::profiles_dir().join(format!("{}.toml", name));
            let profile = Profile::load(&path)?;
            println!("Loaded profile: {} ({})", name, profile.product);
            if let Some(ref key) = profile.key {
                println!("  Key: {}", key);
            }
            println!("  Entitlements: {:?}\n", profile.entitlements);
            return Ok((profile, Some(path)));
        }
    }

    // Create new profile interactively
    println!("Creating new profile...\n");

    let product: String = Input::new()
        .with_prompt("Product identifier")
        .interact_text()?;

    println!("Enter entitlements (one per line, empty to finish):");
    let mut entitlements = Vec::new();
    loop {
        let ent: String = Input::new()
            .with_prompt(format!("  Entitlement {}", entitlements.len() + 1))
            .allow_empty(true)
            .interact_text()?;
        if ent.is_empty() {
            break;
        }
        entitlements.push(ent);
    }

    let profile = Profile { product: product.clone(), entitlements, key: None };
    let path = Profile::profiles_dir().join(format!("{}.toml", product));

    println!();
    Ok((profile, Some(path)))
}

fn verify(pk_path: PathBuf, license_path: PathBuf) -> anyhow::Result<()> {
    use license_guard::LicenseVerifier;

    let pk_hex = fs::read_to_string(&pk_path)?;
    let verifier = LicenseVerifier::from_hex(pk_hex.trim())?;

    let license_data = fs::read_to_string(&license_path)?;

    match verifier.verify_active(&license_data) {
        Ok(payload) => {
            println!("License VALID\n");
            println!("Licensee: {}", payload.sub);
            println!("Product:  {}", payload.iss);
            println!("Issued:   {}", format_timestamp(payload.iat));
            if let Some(exp) = payload.exp {
                println!("Expires:  {}", format_timestamp(exp));
            } else {
                println!("Expires:  Never");
            }
            println!("Entitlements: {:?}", payload.ent);
            if !payload.meta.is_empty() {
                println!("Metadata: {:?}", payload.meta);
            }
        }
        Err(e) => {
            println!("License INVALID: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn show_public_key(key_path: PathBuf) -> anyhow::Result<()> {
    let sk_hex = fs::read_to_string(&key_path)?;
    let sk_bytes = hex::decode(sk_hex.trim())?;
    let sk_bytes: [u8; 32] = sk_bytes.try_into().map_err(|_| anyhow::anyhow!("invalid key"))?;
    let signing_key = SigningKey::from_bytes(&sk_bytes);
    let pk_hex = hex::encode(signing_key.verifying_key().to_bytes());

    println!("Public key (hex):\n{}\n", pk_hex);
    println!("Embed in your app as:");
    println!("  const PUBLIC_KEY: &str = \"{}\";", pk_hex);

    Ok(())
}

fn list_profiles() -> anyhow::Result<()> {
    let profiles = Profile::list_profiles();
    if profiles.is_empty() {
        println!("No profiles found.");
        println!("Profiles are stored in: {}", display_path(&Profile::profiles_dir()));
    } else {
        println!("Available profiles:\n");
        for name in profiles {
            let path = Profile::profiles_dir().join(format!("{}.toml", name));
            if let Ok(profile) = Profile::load(&path) {
                let key_info = profile.key.as_ref().map(|k| format!(" key={}", k)).unwrap_or_default();
                println!("  {} - {}{} entitlements: {:?}", name, profile.product, key_info, profile.entitlements);
                println!("    {}\n", display_path(&path));
            } else {
                println!("  {} (error loading)", name);
                println!("    {}\n", display_path(&path));
            }
        }
    }
    Ok(())
}

fn format_timestamp(ts: u64) -> String {
    chrono::DateTime::from_timestamp(ts as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "invalid".into())
}

/// Format path with ~ for home directory (more readable)
fn display_path(path: &PathBuf) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(suffix) = path.strip_prefix(&home) {
            return format!("~/{}", suffix.display());
        }
    }
    path.display().to_string()
}

/// Collapse home directory to $HOME for portable storage
fn collapse_home(path: &PathBuf) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(suffix) = path.strip_prefix(&home) {
            return format!("$HOME/{}", suffix.display());
        }
    }
    path.display().to_string()
}

/// Expand $HOME in path string to actual home directory
fn expand_home(path: &str) -> PathBuf {
    if path.starts_with("$HOME/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[6..]);
        }
    }
    PathBuf::from(path)
}
