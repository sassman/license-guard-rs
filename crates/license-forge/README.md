# license-forge

CLI tool for generating Ed25519-signed software licenses.

## Commands

### Generate Keypair

```bash
license-forge keygen -o ./keys
```

Creates `license.sk` (private, keep secret) and `license.pk` (public, embed in app).

### Generate License

```bash
license-forge generate -k ./keys/license.sk
```

Interactive prompts for:
- Licensee email/identifier
- Product identifier
- Expiration date (optional)
- Entitlements (reveal, hide, f5, premium)
- Custom metadata (optional)

### Verify License

```bash
license-forge verify -k ./keys/license.pk -l user.lic
```

### Show Public Key

```bash
license-forge show-public-key -k ./keys/license.sk
```

Outputs the public key in hex format for embedding:
```rust
const PUBLIC_KEY: &str = "abc123...";
```

## Workflow

1. **Once**: Generate keypair with `keygen`
2. **Per customer**: Generate license with `generate`
3. **In app**: Embed public key, validate with `license-guard`

## License

MIT OR Apache-2.0
