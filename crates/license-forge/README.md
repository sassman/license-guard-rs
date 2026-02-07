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

First run prompts for:
- Product identifier
- Entitlements (define your own list)
- Option to save as profile

Subsequent runs offer to load saved profiles.

### Load Specific Profile

```bash
license-forge generate -k ./keys/license.sk -p myproduct
```

### List Profiles

```bash
license-forge profiles
```

### Verify License

```bash
license-forge verify -k ./keys/license.pk -l user.lic
```

### Show Public Key

```bash
license-forge show-public-key -k ./keys/license.sk
```

## Profiles

Profiles store product name and available entitlements for reuse.

Location: `~/.config/license-forge/profiles/` (macOS/Linux)

Example profile (`myproduct.toml`):
```toml
product = "myproduct"
entitlements = ["basic", "pro", "enterprise"]
```

## Workflow

1. **Once**: `keygen` to create keypair
2. **First license**: `generate` creates profile interactively
3. **Next licenses**: `generate -p myproduct` loads saved profile
4. **In app**: Embed public key, validate with `license-guard`

## License

MIT OR Apache-2.0
