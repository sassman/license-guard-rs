# license-guard-rs

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

license-forge keygen
    │
    ├─► private key (keep secret!)
    └─► public key ──────────────────► embedded in app
                                             │
license-forge generate ◄── customer info     │
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
