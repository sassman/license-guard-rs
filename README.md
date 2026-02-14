# license-guard-rs

[![License](https://img.shields.io/crates/l/license-guard.svg)](LICENSE-MIT)
[![Build](https://github.com/sassman/license-guard-rs/actions/workflows/build.yml/badge.svg)](https://github.com/sassman/license-guard-rs/actions/workflows/build.yml)

Offline software license validation using Ed25519 signatures.

## Crates

| Crate | Description | Install |
|-------|-------------|---------|
| [license-guard](crates/license-guard/) | Library — validate licenses in your app | `cargo add license-guard` |
| [license-forge](crates/license-forge/) | CLI — generate keypairs and sign licenses | `cargo install license-forge` |

## How It Works

```
Developer                              User
────────                              ────

license-forge product add
    │
    ├─► private key (keep secret!)
    └─► public key ──────────────────► embedded in app
                                             │
license-forge license add ◄── customer info  │
    │                                        │
    └─► license.lic ─────────────────► license-guard validates
                                       with public key
```

## Getting Started

**1. Create a product** (generates Ed25519 keypair):

```bash
cargo install license-forge
license-forge product add my-app
```

**2. Embed the public key** in your Rust app:

```bash
cargo add license-guard
```

```rust
use license_guard::global;

// Public key from: license-forge show
global::init("e8601e48b69383ba520245fd07971e983d06d22c4257cfd82304601479cee788")?;

// Activate when the user provides a license
let license = license_guard::LicenseFile::from_path("license.lic")?;
let data = serde_json::to_string(&license)?;
global::activate(&data)?;

// Check entitlements anywhere
if global::has("premium") {
    // premium feature enabled
}
```

**3. Issue licenses** to users:

```bash
license-forge license add \
  --sub "alice@example.com" \
  --ent "premium,export" \
  --exp 2026-12-31
```

## Security

- **Ed25519 signatures** — 128-bit security, RFC 8032, FIPS 186-5
- **Asymmetric** — the public key cannot forge licenses; only the private key can sign
- **Offline** — no server contact needed for validation
- **Tamper-proof** — any modification to the license payload invalidates the signature

## Known Limitations

- **No revocation** — there is no mechanism to revoke a previously issued, still-valid license
- **No online checks** — fully offline; no server-side revocation list or phone-home
- **No binary protection** — an attacker with write access to the binary can bypass checks

See [SECURITY.md](SECURITY.md) for the full threat model.

## MSRV

The minimum supported Rust version is **1.85**, driven by the Ed25519 cryptographic stack.

## License

MIT OR Apache-2.0
