# Testing

This project is a CLI tool that parses and rewrites OPNsense `config.xml` files. Tests are designed to validate correctness and safety for production use.

## Prerequisites

- Rust toolchain (stable)
- `cargo` and `rustfmt` available on PATH

## Run Tests

```bash
cargo test
```

## Linting

```bash
cargo clippy
```

## Formatting

```bash
cargo fmt
```

## Build

```bash
cargo build --release
```

The binary will be available at `target/release/isc2kea`.

## Manual Validation (Optional)

Run the tool against a real or representative OPNsense `config.xml` to validate end-to-end behavior:

```bash
target/release/isc2kea scan --in ./path/to/config.xml
target/release/isc2kea convert --in ./path/to/config.xml --out /tmp/config.xml.new
```

For dnsmasq:

```bash
target/release/isc2kea scan --in ./path/to/config.xml --backend dnsmasq
target/release/isc2kea convert --in ./path/to/config.xml --out /tmp/config.xml.new --backend dnsmasq
```

Notes:
- The tool is read-only unless `convert --out` is used.
- `example/config.xml` may contain live configuration data; treat it as sensitive.
