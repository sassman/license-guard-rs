# license-forge

[![Crates.io](https://img.shields.io/crates/v/license-forge.svg)](https://crates.io/crates/license-forge)
[![License](https://img.shields.io/crates/l/license-forge.svg)](../../LICENSE-MIT)

CLI tool for generating Ed25519-signed software licenses.

Part of the [license-guard](https://crates.io/crates/license-guard) ecosystem.

## Installation

```bash
cargo install license-forge
```

## Workflow

```
1. Create a product     →  generates Ed25519 keypair
2. Embed the public key →  in your app (via license-guard)
3. Issue licenses        →  signs a payload with the private key
4. Distribute .lic files →  to your users
5. App validates offline →  using the embedded public key
```

### Step-by-step

```bash
# 1. Create a product (generates keypair + config)
license-forge product add my-app

# 2. Copy the public key for embedding
license-forge show
#   Public key (for embedding):
#   e8601e48b69383ba520245fd07971e983d06d22c4257cfd82304601479cee788

# 3. Issue a license to a user
license-forge license add --sub "alice@example.com" --ent "premium,export"

# 4. Verify it works
license-forge verify alice_at_example_com.lic
```

## What's in a License

A license is a JSON file containing a **base64-encoded payload** and its
**Ed25519 signature**. The payload has these fields:

| Field | Type | Description |
|-------|------|-------------|
| `v`   | `u32` | Schema version (currently `1`) |
| `sub` | `string` | **Licensee** — email, user ID, or company name |
| `iss` | `string` | **Issuer** — product identifier (from `product.toml`) |
| `iat` | `u64` | **Issued at** — Unix timestamp (UTC) |
| `exp` | `u64?` | **Expires** — Unix timestamp (UTC), omitted for perpetual |
| `ent` | `[string]` | **Entitlements** — feature flags (e.g. `["premium", "f5"]`) |
| `meta` | `{k: v}` | **Metadata** — arbitrary key-value pairs |

### License file format (`.lic`)

```json
{
  "payload": "eyJ2IjoxLCJzdWIiOiJhbGljZUBleGFtcGxlLmNvbSIsImlzcyI6...==",
  "sig": "MEUCIQD7x8z3y4z5..."
}
```

The payload is the base64-encoded JSON of the fields above. The signature
is the Ed25519 signature over the raw JSON bytes (not the base64).

## Commands

### `product add [name]`

Create a new product with an Ed25519 keypair.

```bash
license-forge product add my-app
```

Prompts interactively for:
- **Product identifier** — appears as `iss` in licenses
- **Entitlements** — predefined list users can pick from when issuing licenses

Creates `~/.config/license-forge/my-app/` with `product.toml`, `license.private`, and `license.pub`.

### `product list`

List all products with their key status and license count.

### `product remove <name>`

Remove a product and all its licenses. Prompts for confirmation.

### `license add`

Generate a new signed license. Runs interactively by default, or accepts
all parameters via CLI flags for scripting:

| Flag | Description | Example |
|------|-------------|---------|
| `--sub` | Licensee email/identifier | `--sub "alice@example.com"` |
| `--exp` | Expiration date (YYYY-MM-DD) | `--exp 2026-12-31` |
| `--ent` | Comma-separated entitlements | `--ent "premium,export"` |
| `--meta` | Comma-separated key=value pairs | `--meta "seats=5,org=Acme"` |

**Interactive mode** (prompts for each field):

```bash
license-forge license add
```

**Scripted mode** (no prompts when all flags are provided):

```bash
license-forge license add \
  --sub "alice@example.com" \
  --exp 2026-12-31 \
  --ent "premium,export" \
  --meta "seats=5,org=Acme Inc"
```

**Perpetual license** (omit `--exp`):

```bash
license-forge license add --sub "alice@example.com" --ent "premium"
```

Saves to `~/.config/license-forge/<product>/licenses/<sub>.lic`.

### `license list`

List all licenses for a product with status (active, expired, perpetual).

### `license expire <name>`

Immediately expire a license (sets `exp` to now, re-signs). Creates a
backup `.bak` file first.

### `license renew <name>`

Extend a license's expiration date. Prompts for a new date. Creates a
backup first.

### `verify <license>`

Verify a license file against the product's public key.

```bash
license-forge verify alice_at_example_com.lic
```

Shows: validity, licensee, issuer, dates, entitlements, and metadata.
Exits with code `1` if invalid.

### `show`

Display product details, public key (for embedding), and license count.

### `completions <shell>`

Generate shell completions (bash, zsh, fish).

```bash
license-forge completions bash >> ~/.bashrc
license-forge completions zsh >> ~/.zshrc
license-forge completions fish > ~/.config/fish/completions/license-forge.fish
```

## Multi-product Support

Use `-p` to target a specific product (default: `default`):

```bash
license-forge -p my-app license add --sub "alice@example.com"
license-forge -p my-app show
license-forge -p my-app license list
```

## Product Configuration

Each product is stored in `~/.config/license-forge/<name>/`:

```
~/.config/license-forge/
├── default/
│   ├── product.toml          # Product configuration
│   ├── license.private       # Ed25519 private key (KEEP SECRET)
│   ├── license.pub           # Ed25519 public key (embed in your app)
│   └── licenses/
│       ├── alice_at_example_com.lic
│       └── bob_at_corp_com.lic
└── my-app/
    └── ...
```

### `product.toml`

```toml
name = "my-app"
entitlements = ["premium", "export", "f5"]
```

| Field | Description |
|-------|-------------|
| `name` | Product identifier — appears as `iss` in every license |
| `entitlements` | Predefined entitlement names — shown as a pick-list during `license add` |

### Key files

- **`license.private`** — hex-encoded 32-byte Ed25519 signing key. Keep
  this secret. If compromised, anyone can forge licenses.
- **`license.pub`** — hex-encoded 32-byte Ed25519 verifying key. Embed
  this in your application source code.

## Integration with license-guard

In your Rust application, add `license-guard` and embed the public key:

```rust
use license_guard::global;

// Public key from: license-forge show
global::init("e8601e48b69383ba520245fd07971e983d06d22c4257cfd82304601479cee788")?;

// Load + activate a license the user provides
let license = license_guard::LicenseFile::from_path("license.lic")?;
let data = serde_json::to_string(&license)?;
global::activate(&data)?;

// Check entitlements anywhere
if global::has("premium") {
    // premium feature
}
```

See the [license-guard docs](https://docs.rs/license-guard) for the full API.

## License

MIT OR Apache-2.0
