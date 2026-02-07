# license-forge

[![Crates.io](https://img.shields.io/crates/v/license-forge.svg)](https://crates.io/crates/license-forge)
[![License](https://img.shields.io/crates/l/license-forge.svg)](../../LICENSE-MIT)

CLI tool for generating Ed25519-signed software licenses.

## Quick Start

```bash
# Create your first license (auto-creates default product)
license-forge license add

# List your products
license-forge product list

# View product details and public key
license-forge show
```

## Commands

### Products

```bash
# Create a new product
license-forge product add myproduct

# List all products
license-forge product list

# Remove a product
license-forge product remove myproduct
```

### Licenses

```bash
# Generate a license (uses default product)
license-forge license add

# Generate a license for specific product
license-forge -p myproduct license add

# List all licenses
license-forge license list

# Expire a license
license-forge license expire user_at_example

# Renew a license
license-forge license renew user_at_example

# Verify a license
license-forge verify user_at_example.lic
```

### Info

```bash
# Show product details, keys, and license count
license-forge show

# Show for specific product
license-forge -p myproduct show
```

## Directory Structure

Products are stored in `~/.config/license-forge/`:

```
~/.config/license-forge/
├── default/
│   ├── product.toml        # Product config
│   ├── license.private     # Private key (keep secret!)
│   ├── license.pub         # Public key (embed in app)
│   └── licenses/
│       └── user_at_example.lic
└── myproduct/
    └── ...
```

## Shell Completions

```bash
# Bash
license-forge completions bash >> ~/.bashrc

# Zsh
license-forge completions zsh >> ~/.zshrc

# Fish
license-forge completions fish > ~/.config/fish/completions/license-forge.fish
```

## Workflow

1. **First license**: `license-forge license add` — creates default product interactively
2. **Additional products**: `license-forge product add myproduct`
3. **In your app**: Embed the public key from `license-forge show`
4. **Validate**: Use `license-guard` crate in your app

## License

MIT OR Apache-2.0
