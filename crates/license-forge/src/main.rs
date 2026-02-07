use base64::prelude::*;
use base64::Engine;
use chrono::{NaiveDate, Utc};
use clap::{Parser, Subcommand};
use dialoguer::{Confirm, Input, MultiSelect};
use ed25519_dalek::{Signer, SigningKey};
use license_guard::{LicenseFile, LicensePayload};
use rand::rngs::OsRng;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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
        /// Path to private key file
        #[arg(short, long)]
        key: PathBuf,
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
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Keygen { output } => keygen(output),
        Commands::Generate { key, output } => generate(key, output),
        Commands::Verify { public_key, license } => verify(public_key, license),
        Commands::ShowPublicKey { key } => show_public_key(key),
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

fn generate(key_path: PathBuf, output: Option<PathBuf>) -> anyhow::Result<()> {
    // Load private key
    let sk_hex = fs::read_to_string(&key_path)?;
    let sk_bytes = hex::decode(sk_hex.trim())?;
    let sk_bytes: [u8; 32] = sk_bytes.try_into().map_err(|_| anyhow::anyhow!("invalid key"))?;
    let signing_key = SigningKey::from_bytes(&sk_bytes);

    println!("License Generator\n");

    // Collect license info via dialoguer
    let sub: String = Input::new()
        .with_prompt("Licensee email/identifier")
        .interact_text()?;

    let iss: String = Input::new()
        .with_prompt("Product identifier")
        .default("stegano".into())
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

    let available_entitlements = vec!["reveal", "hide", "f5", "premium"];
    let selected = MultiSelect::new()
        .with_prompt("Select entitlements")
        .items(&available_entitlements)
        .interact()?;

    let ent: Vec<String> = selected
        .iter()
        .map(|&i| available_entitlements[i].to_string())
        .collect();

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
        iss,
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

    Ok(())
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

fn format_timestamp(ts: u64) -> String {
    chrono::DateTime::from_timestamp(ts as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "invalid".into())
}
