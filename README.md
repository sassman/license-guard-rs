# license-guard-rs

[![License](https://img.shields.io/crates/l/license-guard.svg)](LICENSE-MIT)
[![Build](https://github.com/sassman/license-guard-rs/actions/workflows/build.yml/badge.svg)](https://github.com/sassman/license-guard-rs/actions/workflows/build.yml)

Offline software license validation using Ed25519 signatures.

## Crates

| Crate | Description |
|-------|-------------|
| [license-guard](crates/license-guard/) | Library for validating licenses in your app |
| [license-forge](crates/license-forge/) | CLI tool for generating licenses |

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

## Security

- **Ed25519 signatures** - 128-bit security, FIPS 186-5 approved
- **Asymmetric** - public key can't forge licenses
- **Offline** - no server contact needed for validation

## License

MIT OR Apache-2.0
