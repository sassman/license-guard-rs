//! Example showing how to integrate license-guard into an application.
//!
//! This demonstrates the recommended layered architecture:
//! - license-guard provides generic functionality
//! - Your app provides app-specific helpers (entitlement names, public key)
//!
//! Run with: cargo run -p license-guard --example integration

use license_guard::global;

// =============================================================================
// APPLICATION-SPECIFIC MODULE (would live in your app's src/licensing.rs)
// =============================================================================

mod app_licensing {
    use license_guard::global;

    /// Your public key from license-forge (replace with actual key)
    const PUBLIC_KEY: &str = "e8601e48b69383ba520245fd07971e983d06d22c4257cfd82304601479cee788";

    /// Initialize license system - call once at app startup
    pub fn init() {
        global::init(PUBLIC_KEY).expect("failed to initialize license system");
    }

    // -------------------------------------------------------------------------
    // App-specific entitlement helpers
    // These encapsulate YOUR app's business logic about what features need what
    // -------------------------------------------------------------------------

    /// Check if user can use the "hide" feature
    pub fn can_hide() -> bool {
        global::has("hide") || global::has("premium")
    }

    /// Check if user can use the "reveal" feature (free in freemium model)
    pub fn can_reveal() -> bool {
        true
    }

    /// Check if user can use F5 steganography
    pub fn can_use_f5() -> bool {
        global::has("f5") || global::has("premium")
    }

    /// Check if user has premium tier
    pub fn is_premium() -> bool {
        global::has("premium")
    }
}

// =============================================================================
// EXAMPLE APPLICATION USAGE
// =============================================================================

fn main() {
    println!("=== License Guard Integration Example ===\n");

    // 1. Initialize at startup (typically in main.rs)
    app_licensing::init();
    println!("License system initialized.");

    // 2. Check initial state (no license loaded)
    print_license_status();

    // 3. Simulate user entering a license
    // In a real app, this would come from user input or saved preferences
    println!("\n--- Simulating license activation ---");
    let demo_license = create_demo_license();

    match global::activate(&demo_license) {
        Ok(payload) => {
            println!("License activated!");
            println!("  Licensee: {}", payload.sub);
            println!("  Entitlements: {:?}", payload.ent);
        }
        Err(e) => {
            println!("License activation failed: {}", e);
            println!("  (This is expected - demo license has invalid signature)");
        }
    }

    // 4. Check features from deep in the application
    // Notice: no license object needs to be passed!
    println!("\n--- Feature checks (from anywhere in code) ---");
    simulate_deep_function_call();

    // 5. Deactivate (e.g., user logs out)
    println!("\n--- Deactivating license ---");
    global::deactivate();
    print_license_status();
}

/// Simulates a function deep in the call stack that needs to check licensing
fn simulate_deep_function_call() {
    do_some_work();
}

fn do_some_work() {
    check_features_in_nested_call();
}

fn check_features_in_nested_call() {
    // No license parameter needed - use app_licensing helpers
    println!("  can_hide():    {}", app_licensing::can_hide());
    println!("  can_reveal():  {}", app_licensing::can_reveal());
    println!("  can_use_f5():  {}", app_licensing::can_use_f5());
    println!("  is_premium():  {}", app_licensing::is_premium());
}

fn print_license_status() {
    println!("\nCurrent license status:");
    println!("  is_licensed(): {}", global::is_licensed());
    println!("  licensee():    {:?}", global::licensee());
}

/// Creates a demo license (note: signature is invalid, just for structure demo)
fn create_demo_license() -> String {
    r#"{"payload":"eyJ2IjoxLCJzdWIiOiJkZW1vQGV4YW1wbGUuY29tIiwiaXNzIjoic3RlZ2FubyIsImlhdCI6MTcwNjQwMDAwMCwiZW50IjpbImhpZGUiLCJyZXZlYWwiXX0=","sig":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}"#.to_string()
}
