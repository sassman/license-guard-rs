mod product;

use base64::prelude::*;
use chrono::{NaiveDate, Utc};
use clap::{Parser, Subcommand};
use dialoguer::{Confirm, Input, MultiSelect};
use ed25519_dalek::Signer;
use license_guard::{LicenseFile, LicensePayload};
use product::Product;
use std::collections::HashMap;
use std::fs;

#[derive(Parser)]
#[command(name = "license-forge")]
#[command(about = "Generate Ed25519-signed software licenses")]
struct Cli {
    /// Product to use (default: "default")
    #[arg(short, long, global = true, default_value = "default")]
    product: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage products
    Product {
        #[command(subcommand)]
        action: ProductCommands,
    },
    /// Manage licenses
    License {
        #[command(subcommand)]
        action: LicenseCommands,
    },
    /// Show product details, keys, and licenses
    Show,
    /// Verify a license file
    Verify {
        /// License file to verify (filename or path)
        license: String,
    },
}

#[derive(Subcommand)]
enum ProductCommands {
    /// Create a new product
    Add {
        /// Product name (will prompt if not provided)
        name: Option<String>,
    },
    /// List all products
    List,
    /// Remove a product
    Remove {
        /// Product name to remove
        name: String,
    },
}

#[derive(Subcommand)]
enum LicenseCommands {
    /// Generate a new license
    Add,
    /// List all licenses for the product
    List,
    /// Expire a license (sets expiration to now)
    Expire {
        /// License filename
        license: String,
    },
    /// Renew a license (extend expiration)
    Renew {
        /// License filename
        license: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Product { action } => match action {
            ProductCommands::Add { name } => product_add(name),
            ProductCommands::List => product_list(),
            ProductCommands::Remove { name } => product_remove(&name),
        },
        Commands::License { action } => match action {
            LicenseCommands::Add => license_add(&cli.product),
            LicenseCommands::List => license_list(&cli.product),
            LicenseCommands::Expire { license } => license_expire(&cli.product, &license),
            LicenseCommands::Renew { license } => license_renew(&cli.product, &license),
        },
        Commands::Show => show(&cli.product),
        Commands::Verify { license } => verify(&cli.product, &license),
    }
}

fn product_add(name: Option<String>) -> anyhow::Result<()> {
    // Get product name
    let product_name = if let Some(n) = name {
        n
    } else {
        Input::new()
            .with_prompt("Product name (used as directory name)")
            .interact_text()?
    };

    // Check if already exists
    if Product::exists(&product_name) {
        if Product::has_keys(&product_name) {
            let replace = Confirm::new()
                .with_prompt(format!(
                    "Product '{}' already has keys. Replace them? (old keys will be backed up)",
                    product_name
                ))
                .default(false)
                .interact()?;

            if !replace {
                println!("Aborted.");
                return Ok(());
            }
            Product::backup_keys(&product_name)?;
            println!("Existing keys backed up.\n");
        }
    }

    println!("\nConfiguring product '{}'...\n", product_name);

    // Get product display name
    let display_name: String = Input::new()
        .with_prompt("Product identifier (appears in licenses)")
        .default(product_name.clone())
        .interact_text()?;

    // Get entitlements
    println!("\nEnter entitlements (empty line to finish):");
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

    // Create and save product
    let product = Product {
        name: display_name,
        entitlements,
    };
    product.save(&product_name)?;

    // Generate keys
    println!("\nGenerating keys...");
    let (_, pk_hex) = Product::generate_keys(&product_name)?;

    // Show summary
    let dir = Product::dir(&product_name);
    println!("\n Product '{}' created successfully!\n", product_name);
    println!("  Directory:   {}", Product::display_path(&dir));
    println!(
        "  Config:      {}",
        Product::display_path(&Product::config_path(&product_name))
    );
    println!(
        "  Private key: {}",
        Product::display_path(&Product::private_key_path(&product_name))
    );
    println!(
        "  Public key:  {}",
        Product::display_path(&Product::public_key_path(&product_name))
    );
    println!("\n  Public key (for embedding in your app):");
    println!("  {}\n", pk_hex);

    if product_name != "default" {
        println!("  Use with: license-forge -p {} license add", product_name);
    } else {
        println!("  Use with: license-forge license add");
    }

    Ok(())
}

fn product_list() -> anyhow::Result<()> {
    todo!("product_list")
}

fn product_remove(_name: &str) -> anyhow::Result<()> {
    todo!("product_remove")
}

fn license_add(_product: &str) -> anyhow::Result<()> {
    todo!("license_add")
}

fn license_list(_product: &str) -> anyhow::Result<()> {
    todo!("license_list")
}

fn license_expire(_product: &str, _license: &str) -> anyhow::Result<()> {
    todo!("license_expire")
}

fn license_renew(_product: &str, _license: &str) -> anyhow::Result<()> {
    todo!("license_renew")
}

fn show(_product: &str) -> anyhow::Result<()> {
    todo!("show")
}

fn verify(_product: &str, _license: &str) -> anyhow::Result<()> {
    todo!("verify")
}

fn format_timestamp(ts: u64) -> String {
    chrono::DateTime::from_timestamp(ts as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "invalid".into())
}
