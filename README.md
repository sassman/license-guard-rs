# license-guard-rs

Offline software license validation using Ed25519 signatures.

## Crates

- **license-guard** - Library for validating licenses in your application
- **license-forge** - CLI tool for generating licenses (developer-side)

## Quick Start

```rust
use license_guard::global;

// Initialize once at startup
global::init("your_public_key_hex")?;

// Activate when user enters license
global::activate(license_data)?;

// Check entitlements from anywhere
if global::has("premium") {
    // premium feature enabled
}
```

## Security

Uses Ed25519 asymmetric signatures:
- Private key stays on your server (generates licenses)
- Public key embedded in app (verifies licenses)
- Keygens are cryptographically impossible without private key

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
