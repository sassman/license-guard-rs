# license-guard

[![Crates.io](https://img.shields.io/crates/v/license-guard.svg)](https://crates.io/crates/license-guard)
[![Documentation](https://docs.rs/license-guard/badge.svg)](https://docs.rs/license-guard)
[![License](https://img.shields.io/crates/l/license-guard.svg)](../../LICENSE-MIT)

Offline license validation using Ed25519 signatures.

Part of the [license-guard](https://github.com/sassman/license-guard-rs) ecosystem.
Licenses are generated with the [`license-forge`](https://crates.io/crates/license-forge) CLI.

## Installation

```bash
cargo add license-guard
```

## Quick Start

```rust
use license_guard::global;

// 1. Initialize once at startup with your public key
global::init("your_public_key_hex")?;

// 2. Load and activate when user provides a license
let license = license_guard::LicenseFile::from_path("license.lic")?;
let data = serde_json::to_string(&license)?;
global::activate(&data)?;

// 3. Check entitlements anywhere
if global::has("premium") {
    // premium feature enabled
}
```

## Choosing an API

| Scenario | API | Why |
|----------|-----|-----|
| Most apps (single product) | [`global`](https://docs.rs/license-guard/latest/license_guard/global/) module | No context passing, init-once |
| Multiple products / tenants | [`LicenseVerifier`](https://docs.rs/license-guard/latest/license_guard/struct.LicenseVerifier.html) | One instance per product |
| Unit tests | [`LicenseState`](https://docs.rs/license-guard/latest/license_guard/global/struct.LicenseState.html) | No global state, fully parallel |

### Direct API

```rust
use license_guard::LicenseVerifier;

let verifier = LicenseVerifier::from_hex("abc123...")?;
let payload = verifier.verify_active(license_data)?;
println!("Licensed to: {}", payload.sub);
```

## Verification Methods

Methods are layered — each adds a check on top of the previous:

| Method | Signature | Expiry | Entitlement |
|--------|-----------|--------|-------------|
| `verify` | yes | — | — |
| `verify_active` | yes | yes | — |
| `verify_with_entitlement` | yes | yes | yes |

`verify_auto` is a format-detecting wrapper around `verify` — use it when
the input may be JSON or compact format (e.g. loaded from a file).

## Loading Licenses

| Source | Method |
|--------|--------|
| File on disk | `LicenseFile::from_path()` — auto-detects JSON / compact |
| User text input (base64) | `LicenseFile::from_base64()` — for paste-in license keys |
| Raw JSON or compact string | `LicenseVerifier::verify_auto()` — auto-detects format |

### License file formats

**JSON** (default `.lic` format):

```json
{
  "payload": "eyJ2IjoxLCJzdWIiOiJhbGljZUBleGFtcGxlLmNvbSIs...==",
  "sig": "MEUCIQD7x8z3y4z5..."
}
```

**Compact** (single line, dot-separated):

```
eyJ2IjoxLCJzdWIiOiJhbGljZUBleGFtcGxlLmNvbSIs...==.MEUCIQD7x8z3y4z5...
```

## Error Handling

All fallible operations return `LicenseError`. Match on variants to give
users actionable feedback:

```rust
use license_guard::LicenseError;

match verifier.verify_active(license_data) {
    Ok(payload) => println!("Licensed to {}", payload.sub),
    Err(LicenseError::InvalidSignature) => eprintln!("License is invalid or tampered"),
    Err(LicenseError::Expired(ts)) => eprintln!("License expired at {ts}"),
    Err(LicenseError::MissingEntitlement(e)) => eprintln!("Missing entitlement: {e}"),
    Err(e) => eprintln!("License error: {e}"),
}
```

## License Payload Fields

| Field | Type | Description |
|-------|------|-------------|
| `v` | `u32` | Schema version (currently `1`) |
| `sub` | `String` | Licensee — email, user ID, or company name |
| `iss` | `String` | Product identifier (matches `license-forge` product name) |
| `iat` | `u64` | Issued-at timestamp (Unix seconds, UTC) |
| `exp` | `Option<u64>` | Expiry timestamp — `None` means perpetual (never expires) |
| `ent` | `Vec<String>` | Entitlements — feature flags (e.g. `["premium", "export"]`) |
| `meta` | `HashMap<String, String>` | Arbitrary key-value metadata (e.g. `seats`, `org`) |

## Features

- **Offline** — no network calls, no server dependency
- **Ed25519** — 128-bit security, RFC 8032, FIPS 186-5
- **Perpetual licenses** — omit `exp` for licenses that never expire
- **Entitlement-based** — gate features with arbitrary string flags
- **Multiple formats** — JSON and compact (dot-separated) license files
- **Base64 input** — validate pasted license keys before persisting
- **Thread-safe** — `global` module uses `RwLock`, safe for concurrent access

## License

MIT OR Apache-2.0
