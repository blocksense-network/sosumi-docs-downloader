# Sosumi Docs Downloader

A fast, concurrent documentation downloader for [sosumi.ai](https://sosumi.ai) that preserves proper markdown link relationships and supports downloading specific documentation paths or entire documentation types.

## Features

- 🚀 **Concurrent downloads** using async Rust with tokio
- 🔗 **Link preservation** - converts absolute URLs to relative paths for offline browsing
- 📚 **Flexible paths** - download specific documentation pages or entire documentation types
- 🎯 **Precise link parsing** - handles complex URLs with nested parentheses
- 🏗️ **Nix-powered** - reproducible builds and development environment

## Installation

### Using Nix (Recommended)

1. **Clone the repository:**
   ```bash
   git clone <repository-url>
   cd sosumi-docs-downloader
   ```

2. **Enter the development environment:**
   ```bash
   nix develop
   ```

3. **Build the binary:**
   ```bash
   nix build
   ```

4. **Run directly from nix:**
   ```bash
   nix run . -- endpointsecurity
   ```

### Manual Build

If you have Rust installed:

```bash
cargo build --release
./target/release/sosumi-docs-downloader --help
```

## Usage

### Basic Usage

Download a single documentation type:
```bash
sosumi-docs-downloader endpointsecurity
```

Download multiple documentation types:
```bash
sosumi-docs-downloader endpointsecurity fskit sandboxing
```

### Advanced Options

```bash
sosumi-docs-downloader [OPTIONS] <DOC_TYPES>...

# Download to specific directory
sosumi-docs-downloader -o ./docs endpointsecurity

# Limit crawl depth (default: unlimited)
sosumi-docs-downloader -d 2 endpointsecurity

# Combine options
sosumi-docs-downloader -o ./my-docs -d 3 endpointsecurity fskit
```

### Command Line Options

- `<DOC_TYPES>` - Documentation paths to download (required)
  - Examples: `endpointsecurity`, `swift/array`, `fskit`, `sandboxing`, etc.
  - Can be top-level documentation types or specific sub-paths

- `-o, --outdir <DIR>` - Output directory (default: current directory `.`)

- `-d, --max-depth <NUM>` - Maximum crawl depth (default: 0 = unlimited)
  - Use `1` for single page only
  - Use `2` for moderate depth
  - Use `0` for unlimited depth (recommended)

### Examples

**Download Endpoint Security docs to current directory:**
```bash
sosumi-docs-downloader endpointsecurity
```

**Download multiple docs with depth limit:**
```bash
sosumi-docs-downloader -d 2 endpointsecurity fskit
```

**Download specific documentation page:**
```bash
sosumi-docs-downloader swift/array
```

**Download to custom directory:**
```bash
sosumi-docs-downloader -o ./documentation endpointsecurity
```

**Full example with all options:**
```bash
sosumi-docs-downloader -o ./docs -d 3 endpointsecurity fskit sandboxing
```

## Output Structure

The downloader creates a directory structure like:
```
output-dir/
├── endpointsecurity/
│   ├── index.md
│   ├── client.md
│   ├── es_new_client(_:_:).md
│   └── ...
└── fskit/
    ├── index.md
    └── ...
```

All markdown links are converted from absolute URLs to relative paths, making the documentation browsable offline.

## Development

### Prerequisites

- [Nix](https://nixos.org/download.html) (recommended)
- Or [Rust](https://rustup.rs/) 1.70+

### Development Workflow

```bash
# Enter development environment
nix develop

# Run tests
cargo test

# Build and run
cargo run -- endpointsecurity

# Format code
cargo fmt

# Lint code
cargo clippy
```

### Nix Flake Commands

```bash
# Build the package
nix build

# Run directly
nix run . -- endpointsecurity

# Enter development shell
nix develop

# Check the flake
nix flake check
```

## Technical Details

- **Language:** Rust with async/await
- **HTTP Client:** reqwest with connection pooling
- **Concurrency:** tokio with futures for parallel downloads
- **Link Parsing:** Custom regex-free parser handling nested parentheses
- **Build System:** Nix flakes for reproducible builds

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test with `nix build && nix run . -- endpointsecurity`
5. Submit a pull request

## License

[MIT License](LICENSE)

## Related Projects

- [sosumi.ai](https://sosumi.ai) - The documentation source
- [agents-workflow](https://github.com/blocksense-network/agents-workflow) - Parent project