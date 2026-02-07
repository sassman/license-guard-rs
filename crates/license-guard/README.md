# license-guard

Offline license validation using Ed25519 signatures.

## Quick Start

```rust
use license_guard::global;

// 1. Initialize once at startup
global::init("your_public_key_hex")?;

// 2. Activate when user enters license
global::activate(license_data)?;

// 3. Check entitlements anywhere
if global::has("premium") {
    // premium feature enabled
}
```

## API Options

| Use case | API |
|----------|-----|
| Most apps | `global` module - no context passing |
| Multiple products | `LicenseVerifier` - one per product |
| Testing | `LicenseVerifier` - no global state |

## Direct API

```rust
use license_guard::LicenseVerifier;

let verifier = LicenseVerifier::from_hex("abc123...")?;
let payload = verifier.verify(license_data)?;
println!("Licensed to: {}", payload.sub);
```

## License Payload Fields

| Field | Description |
|-------|-------------|
| `v` | Schema version (currently 1) |
| `sub` | Licensee identifier (email, user ID) |
| `iss` | Product identifier |
| `iat` | Issue timestamp (Unix seconds) |
| `exp` | Expiry timestamp (optional) |
| `ent` | Enabled features list |
| `meta` | Custom key-value data |

## License

MIT OR Apache-2.0
