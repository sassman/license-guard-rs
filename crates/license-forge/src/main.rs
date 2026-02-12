mod product;

use base64::prelude::*;
use chrono::{Datelike, NaiveDate, Utc};
use clap::{CommandFactory, Parser, Subcommand};
use dialoguer::{Confirm, Input, MultiSelect, Select};
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
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
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
    Add {
        /// Licensee email/identifier (skips interactive prompt)
        #[arg(long)]
        sub: Option<String>,
        /// Expiration date as YYYY-MM-DD (skips interactive prompt)
        #[arg(long)]
        exp: Option<String>,
        /// Comma-separated entitlements (skips interactive prompt)
        #[arg(long)]
        ent: Option<String>,
        /// Comma-separated key=value metadata pairs (skips interactive prompt)
        #[arg(long)]
        meta: Option<String>,
    },
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
            LicenseCommands::Add {
                sub,
                exp,
                ent,
                meta,
            } => license_add(&cli.product, sub, exp, ent, meta),
            LicenseCommands::List => license_list(&cli.product),
            LicenseCommands::Expire { license } => license_expire(&cli.product, &license),
            LicenseCommands::Renew { license } => license_renew(&cli.product, &license),
        },
        Commands::Show => show(&cli.product),
        Commands::Verify { license } => verify(&cli.product, &license),
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "license-forge", &mut std::io::stdout());
            Ok(())
        }
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
    if Product::exists(&product_name) && Product::has_keys(&product_name) {
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
    let (_, pub_hex) = Product::generate_keys(&product_name)?;

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
    println!("  {}\n", pub_hex);

    if product_name != "default" {
        println!("  Use with: license-forge -p {} license add", product_name);
    } else {
        println!("  Use with: license-forge license add");
    }

    Ok(())
}

fn product_list() -> anyhow::Result<()> {
    let products = Product::list();

    if products.is_empty() {
        println!("No products configured.");
        println!("\nCreate one with: license-forge product add");
        return Ok(());
    }

    println!("Products:\n");
    for name in products {
        let dir = Product::dir(&name);
        if let Ok(product) = Product::load(&name) {
            let key_status = if Product::has_keys(&name) {
                "✓"
            } else {
                "✗"
            };
            let license_count = count_licenses(&name);
            println!("  {} [{}]", name, key_status);
            println!("    Name: {}", product.name);
            println!("    Entitlements: {:?}", product.entitlements);
            println!("    Licenses: {}", license_count);
            println!("    Path: {}", Product::display_path(&dir));
            println!();
        } else {
            println!("  {} (error loading)", name);
            println!("    Path: {}\n", Product::display_path(&dir));
        }
    }

    Ok(())
}

fn count_licenses(product_name: &str) -> usize {
    let dir = Product::licenses_dir(product_name);
    if !dir.exists() {
        return 0;
    }
    fs::read_dir(dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "lic")
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

/// Print helpful message when default product doesn't exist
fn print_no_default_product_hint() {
    let products = Product::list();
    if products.is_empty() {
        println!("No products configured.");
        println!("\nCreate one with: license-forge product add");
    } else {
        println!("Specify a product with -p <name>:\n");
        for name in &products {
            println!("  license-forge -p {} <command>", name);
        }
        println!("\nOr create a default product:\n");
        println!("  license-forge product add default");
    }
}

fn product_remove(name: &str) -> anyhow::Result<()> {
    if !Product::exists(name) {
        anyhow::bail!("Product '{}' not found", name);
    }

    let license_count = count_licenses(name);
    let warning = if license_count > 0 {
        format!(" This will delete {} license(s)!", license_count)
    } else {
        String::new()
    };

    let confirm = Confirm::new()
        .with_prompt(format!("Remove product '{}'?{}", name, warning))
        .default(false)
        .interact()?;

    if !confirm {
        println!("Aborted.");
        return Ok(());
    }

    Product::remove(name)?;
    println!("Product '{}' removed.", name);

    Ok(())
}

/// Ensures the product exists. If product is "default" and doesn't exist, creates it interactively.
/// Returns the product name to use.
fn ensure_product_exists(product_name: &str) -> anyhow::Result<String> {
    if Product::exists(product_name) {
        return Ok(product_name.to_string());
    }

    if product_name != "default" {
        anyhow::bail!(
            "Product '{}' not found. Create it with: license-forge product add {}",
            product_name,
            product_name
        );
    }

    // Lazy create default product
    println!("No products configured. Let's set up the default product.\n");

    let display_name: String = Input::new()
        .with_prompt("Product identifier (appears in licenses)")
        .default("default".to_string())
        .interact_text()?;

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

    let product = Product {
        name: display_name,
        entitlements,
    };
    product.save("default")?;

    println!("\nGenerating keys...");
    let (_, pub_hex) = Product::generate_keys("default")?;

    let dir = Product::dir("default");
    println!("\n Default product created!\n");
    println!("  Directory:   {}", Product::display_path(&dir));
    println!("  Public key:  {}\n", pub_hex);

    Ok("default".to_string())
}

/// Prompt user for expiration date with predefined options
fn prompt_expiration_date() -> anyhow::Result<u64> {
    let now = Utc::now();
    let current_year = now.year();
    let end_of_year = format!("End of {}", current_year);

    let options = vec!["+1 Month", "+6 Months", "+1 Year", &end_of_year, "Custom"];

    let selection = Select::new()
        .with_prompt("Expiration")
        .items(&options)
        .default(0)
        .interact()?;

    let date = match selection {
        0 => now.date_naive() + chrono::Months::new(1),
        1 => now.date_naive() + chrono::Months::new(6),
        2 => now.date_naive() + chrono::Months::new(12),
        3 => NaiveDate::from_ymd_opt(current_year, 12, 31).unwrap(),
        4 => {
            let date_str: String = Input::new()
                .with_prompt("Expiration date (YYYY-MM-DD)")
                .interact_text()?;
            NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?
        }
        _ => unreachable!(),
    };

    let datetime = date.and_hms_opt(23, 59, 59).unwrap();
    Ok(datetime.and_utc().timestamp() as u64)
}

/// Parse a YYYY-MM-DD date string into a Unix timestamp (end of day UTC).
fn parse_expiration_date(date_str: &str) -> anyhow::Result<u64> {
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("Invalid date '{}': {} (expected YYYY-MM-DD)", date_str, e))?;
    let datetime = date
        .and_hms_opt(23, 59, 59)
        .ok_or_else(|| anyhow::anyhow!("Invalid date '{}'", date_str))?;
    Ok(datetime.and_utc().timestamp() as u64)
}

/// Parse comma-separated key=value pairs into a HashMap.
fn parse_meta(meta_str: &str) -> anyhow::Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    for pair in meta_str.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let (key, value) = pair
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("Invalid metadata '{}': expected key=value", pair))?;
        map.insert(key.trim().to_string(), value.trim().to_string());
    }
    Ok(map)
}

fn license_add(
    product_name: &str,
    cli_sub: Option<String>,
    cli_exp: Option<String>,
    cli_ent: Option<String>,
    cli_meta: Option<String>,
) -> anyhow::Result<()> {
    let product_name = ensure_product_exists(product_name)?;
    let product = Product::load(&product_name)?;
    let signing_key = Product::load_signing_key(&product_name)?;

    println!("Creating license for product: {}\n", product.name);

    // Collect license info — use CLI args when provided, otherwise prompt
    let sub = if let Some(sub) = cli_sub {
        sub
    } else {
        Input::new()
            .with_prompt("Licensee email/identifier")
            .interact_text()?
    };

    let exp = if let Some(exp_str) = cli_exp {
        Some(parse_expiration_date(&exp_str)?)
    } else {
        let has_expiry = Confirm::new()
            .with_prompt("Set expiration date?")
            .default(false)
            .interact()?;
        if has_expiry {
            Some(prompt_expiration_date()?)
        } else {
            None
        }
    };

    let ent = if let Some(ent_str) = cli_ent {
        ent_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else if product.entitlements.is_empty() {
        println!("Enter entitlements (empty line to finish):");
        let mut entitlements = Vec::new();
        loop {
            let e: String = Input::new()
                .with_prompt(format!("  Entitlement {}", entitlements.len() + 1))
                .allow_empty(true)
                .interact_text()?;
            if e.is_empty() {
                break;
            }
            entitlements.push(e);
        }
        entitlements
    } else {
        let selected = MultiSelect::new()
            .with_prompt("Select entitlements")
            .items(&product.entitlements)
            .interact()?;
        selected
            .iter()
            .map(|&i| product.entitlements[i].clone())
            .collect()
    };

    let meta = if let Some(meta_str) = cli_meta {
        parse_meta(&meta_str)?
    } else {
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
                    .with_prompt(format!("Value for '{}'", key))
                    .interact_text()?;
                meta.insert(key, value);
            }
        }
        meta
    };

    // Create payload
    let now = Utc::now().timestamp() as u64;
    let payload = LicensePayload {
        v: 1,
        sub: sub.clone(),
        iss: product.name.clone(),
        iat: now,
        exp,
        ent,
        meta,
    };

    // Sign payload
    let license_file = payload.sign(&signing_key)?;
    let license_json = serde_json::to_string_pretty(&license_file)?;

    // Save to licenses directory
    let filename = format!("{}.lic", sub.replace('@', "_at_").replace('.', "_"));
    let license_path = Product::licenses_dir(&product_name).join(&filename);

    // Ensure licenses directory exists
    fs::create_dir_all(Product::licenses_dir(&product_name))?;
    fs::write(&license_path, &license_json)?;

    println!("\n License created!\n");
    println!("  Saved to: {}", Product::display_path(&license_path));
    println!("\n--- License Preview ---");
    println!("{}", serde_json::to_string_pretty(&payload)?);

    Ok(())
}

fn license_list(product_name: &str) -> anyhow::Result<()> {
    if !Product::exists(product_name) {
        if product_name == "default" {
            print_no_default_product_hint();
        } else {
            println!("Product '{}' not found.", product_name);
        }
        return Ok(());
    }

    let licenses_dir = Product::licenses_dir(product_name);
    if !licenses_dir.exists() {
        println!("No licenses found for product '{}'.", product_name);
        return Ok(());
    }

    let mut licenses: Vec<_> = fs::read_dir(&licenses_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "lic")
                .unwrap_or(false)
        })
        .collect();

    if licenses.is_empty() {
        println!("No licenses found for product '{}'.", product_name);
        return Ok(());
    }

    // Sort by modification time (newest first)
    licenses.sort_by(|a, b| {
        let a_time = a.metadata().and_then(|m| m.modified()).ok();
        let b_time = b.metadata().and_then(|m| m.modified()).ok();
        b_time.cmp(&a_time)
    });

    println!("Licenses for '{}':\n", product_name);

    for entry in licenses {
        let path = entry.path();
        let filename = path.file_name().unwrap().to_string_lossy();

        // Try to read and parse the license
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(license_file) = serde_json::from_str::<LicenseFile>(&content) {
                if let Ok(payload_bytes) = BASE64_STANDARD.decode(&license_file.payload) {
                    if let Ok(payload) = serde_json::from_slice::<LicensePayload>(&payload_bytes) {
                        let status = match payload.exp {
                            Some(exp) if exp < Utc::now().timestamp() as u64 => " [EXPIRED]",
                            Some(_) => "",
                            None => " [perpetual]",
                        };
                        let exp_str = payload
                            .exp
                            .map(format_timestamp)
                            .unwrap_or_else(|| "never".to_string());

                        println!("  {}{}", filename, status);
                        println!("    Licensee: {}", payload.sub);
                        println!("    Issued:   {}", format_timestamp(payload.iat));
                        println!("    Expires:  {}", exp_str);
                        println!("    Entitlements: {:?}", payload.ent);
                        println!();
                        continue;
                    }
                }
            }
        }
        // Fallback if can't parse
        println!("  {} (unable to parse)", filename);
        println!();
    }

    Ok(())
}

/// Backup a license file with timestamp
fn backup_license(product_name: &str, license_name: &str) -> anyhow::Result<()> {
    let license_path = resolve_license_path(product_name, license_name)?;
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let backup_name = format!(
        "{}.{}.bak",
        license_path.file_stem().unwrap().to_string_lossy(),
        timestamp
    );
    let backup_path = license_path.parent().unwrap().join(backup_name);
    fs::copy(&license_path, &backup_path)?;
    println!("  Backed up to: {}", Product::display_path(&backup_path));
    Ok(())
}

/// Resolve license name to full path (handles both filename and full path)
fn resolve_license_path(
    product_name: &str,
    license_name: &str,
) -> anyhow::Result<std::path::PathBuf> {
    let path = std::path::PathBuf::from(license_name);

    // If it's already a valid path, use it
    if path.exists() {
        return Ok(path);
    }

    // Otherwise, look in the product's licenses directory
    let licenses_dir = Product::licenses_dir(product_name);
    let license_path = if license_name.ends_with(".lic") {
        licenses_dir.join(license_name)
    } else {
        licenses_dir.join(format!("{}.lic", license_name))
    };

    if license_path.exists() {
        Ok(license_path)
    } else {
        anyhow::bail!("License '{}' not found", license_name)
    }
}

/// Load, modify, re-sign, and save a license
fn modify_license<F>(product_name: &str, license_name: &str, modifier: F) -> anyhow::Result<()>
where
    F: FnOnce(&mut LicensePayload) -> anyhow::Result<()>,
{
    let license_path = resolve_license_path(product_name, license_name)?;
    let signing_key = Product::load_signing_key(product_name)?;

    // Read and parse
    let content = fs::read_to_string(&license_path)?;
    let license_file: LicenseFile = serde_json::from_str(&content)?;
    let payload_bytes = BASE64_STANDARD.decode(&license_file.payload)?;
    let mut payload: LicensePayload = serde_json::from_slice(&payload_bytes)?;

    // Apply modification
    modifier(&mut payload)?;

    // Re-sign
    let new_license = payload.sign(&signing_key)?;

    // Save
    fs::write(&license_path, serde_json::to_string_pretty(&new_license)?)?;

    Ok(())
}

fn license_expire(product_name: &str, license_name: &str) -> anyhow::Result<()> {
    let product_name = ensure_product_exists(product_name)?;

    println!("Expiring license: {}\n", license_name);
    backup_license(&product_name, license_name)?;

    let now = Utc::now().timestamp() as u64;
    modify_license(&product_name, license_name, |payload| {
        payload.exp = Some(now.saturating_sub(1)); // Set to 1 second ago
        Ok(())
    })?;

    println!("\n License expired successfully.");
    Ok(())
}

fn license_renew(product_name: &str, license_name: &str) -> anyhow::Result<()> {
    let product_name = ensure_product_exists(product_name)?;

    println!("Renewing license: {}\n", license_name);

    let new_exp = prompt_expiration_date()?;

    backup_license(&product_name, license_name)?;

    modify_license(&product_name, license_name, |payload| {
        payload.exp = Some(new_exp);
        Ok(())
    })?;

    println!("\n License renewed until {}.", format_timestamp(new_exp));
    Ok(())
}

fn show(product_name: &str) -> anyhow::Result<()> {
    if !Product::exists(product_name) {
        if product_name == "default" {
            print_no_default_product_hint();
        } else {
            println!("Product '{}' not found.", product_name);
        }
        return Ok(());
    }

    let product = Product::load(product_name)?;
    let dir = Product::dir(product_name);

    println!("Product: {}\n", product_name);
    println!("  Name:         {}", product.name);
    println!("  Directory:    {}", Product::display_path(&dir));
    println!("  Entitlements: {:?}", product.entitlements);

    // Keys
    if Product::has_keys(product_name) {
        let pub_hex = Product::get_public_key_hex(product_name)?;
        println!("\n  Keys:");
        println!(
            "    Private: {}",
            Product::display_path(&Product::private_key_path(product_name))
        );
        println!(
            "    Public:  {}",
            Product::display_path(&Product::public_key_path(product_name))
        );
        println!("\n  Public key (for embedding):");
        println!("    {}", pub_hex);
    } else {
        println!("\n  Keys: not generated");
    }

    // Licenses summary
    let license_count = count_licenses(product_name);
    println!("\n  Licenses: {}", license_count);
    if license_count > 0 {
        println!(
            "    Directory: {}",
            Product::display_path(&Product::licenses_dir(product_name))
        );
        println!("\n    Run 'license-forge license list' for details.");
    }

    Ok(())
}

fn verify(product_name: &str, license_name: &str) -> anyhow::Result<()> {
    use license_guard::LicenseVerifier;

    let license_path = resolve_license_path(product_name, license_name)?;
    let pub_hex = Product::get_public_key_hex(product_name)?;

    let verifier = LicenseVerifier::from_hex(&pub_hex)?;
    let license_data = fs::read_to_string(&license_path)?;

    match verifier.verify_active(&license_data) {
        Ok(payload) => {
            println!(" License VALID\n");
            println!("  Licensee:     {}", payload.sub);
            println!("  Product:      {}", payload.iss);
            println!("  Issued:       {}", format_timestamp(payload.iat));
            if let Some(exp) = payload.exp {
                println!("  Expires:      {}", format_timestamp(exp));
            } else {
                println!("  Expires:      Never");
            }
            println!("  Entitlements: {:?}", payload.ent);
            if !payload.meta.is_empty() {
                println!("  Metadata:     {:?}", payload.meta);
            }
        }
        Err(e) => {
            println!(" License INVALID: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn format_timestamp(ts: u64) -> String {
    chrono::DateTime::from_timestamp(ts as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "invalid".into())
}
