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

## Extended Manual Validation (Optional)

For more realistic coverage, run a small flag matrix against a real config and
verify outputs are valid XML without leaking sensitive data. Suggested set:

Scan:

```bash
isc2kea scan --in ./path/to/config.xml --backend kea
isc2kea scan --in ./path/to/config.xml --backend kea --create-subnets
isc2kea scan --in ./path/to/config.xml --backend kea --create-options
isc2kea scan --in ./path/to/config.xml --backend kea --create-subnets --create-options

isc2kea scan --in ./path/to/config.xml --backend dnsmasq
isc2kea scan --in ./path/to/config.xml --backend dnsmasq --create-subnets
isc2kea scan --in ./path/to/config.xml --backend dnsmasq --create-options
isc2kea scan --in ./path/to/config.xml --backend dnsmasq --create-subnets --create-options
```

Convert:

```bash
isc2kea convert --in ./path/to/config.xml --out /tmp/config.kea.xml --backend kea --force
isc2kea convert --in ./path/to/config.xml --out /tmp/config.kea.subnets.xml --backend kea --create-subnets --force
isc2kea convert --in ./path/to/config.xml --out /tmp/config.kea.options.xml --backend kea --create-options --force
isc2kea convert --in ./path/to/config.xml --out /tmp/config.kea.all.xml --backend kea --create-subnets --create-options --force
isc2kea convert --in ./path/to/config.xml --out /tmp/config.kea.enable.xml --backend kea --create-subnets --enable-backend --force
isc2kea convert --in ./path/to/config.xml --out /tmp/config.kea.enable.all.xml --backend kea --create-subnets --create-options --enable-backend --force

isc2kea convert --in ./path/to/config.xml --out /tmp/config.dnsmasq.xml --backend dnsmasq --force
isc2kea convert --in ./path/to/config.xml --out /tmp/config.dnsmasq.subnets.xml --backend dnsmasq --create-subnets --force
isc2kea convert --in ./path/to/config.xml --out /tmp/config.dnsmasq.options.xml --backend dnsmasq --create-options --force
isc2kea convert --in ./path/to/config.xml --out /tmp/config.dnsmasq.all.xml --backend dnsmasq --create-subnets --create-options --force
isc2kea convert --in ./path/to/config.xml --out /tmp/config.dnsmasq.enable.xml --backend dnsmasq --create-subnets --enable-backend --force
isc2kea convert --in ./path/to/config.xml --out /tmp/config.dnsmasq.enable.all.xml --backend dnsmasq --create-subnets --create-options --enable-backend --force
```

Validation tips (without exposing content):

- Ensure each `convert` output is non-empty, well-formed XML.
- For `--create-subnets`:
  - Kea outputs should add `<subnet4>`/`<subnet6>` entries.
  - dnsmasq outputs should add `<dhcp_ranges>` entries.
- For `--enable-backend`:
  - ISC `<enable>` values should be cleared on migrated interfaces.
  - Target backend should be enabled (`<enabled>1</enabled>` for Kea; `<enable>1</enable>` for dnsmasq).

### Validation Checks (No Sensitive Output)

For `--enable-backend` runs, verify the output XML only by boolean checks (no values):

- If Kea subnets were created (`<subnet4>`/`<subnet6>` present), ensure:
  - `<Kea><dhcp4><general><enabled>` is `1` when `<subnet4>` exists
  - `<Kea><dhcp6><general><enabled>` is `1` when `<subnet6>` exists
- If dnsmasq ranges were created (`<dhcp_ranges>` present), ensure:
  - `<dnsmasq><enable>` is `1`
- For ISC disablement:
  - At least one `<dhcpd><iface><enable>` is empty
  - At least one `<dhcpdv6><iface><enable>` is empty (if v6 was migrated)

These checks can be done by scanning tag presence and boolean values only.

Example (no sensitive values):

```bash
python - <<'PY'
import xml.etree.ElementTree as ET
from pathlib import Path

def text(elem):
    if elem is None or elem.text is None:
        return ""
    return elem.text.strip()

outdir = Path("/tmp/isc2kea-live")
files = sorted(outdir.glob("*enable_backend*.xml"))
if not files:
    raise SystemExit("No enable-backend outputs found")

print("Enable-backend validation (conditional, no sensitive values):")
all_ok = True
for f in files:
    root = ET.parse(f).getroot()
    base = f.name
    backend = "dnsmasq" if "dnsmasq" in base else "kea"

    has_subnet4 = root.find(".//subnet4") is not None
    has_subnet6 = root.find(".//subnet6") is not None
    has_ranges = root.find(".//dhcp_ranges") is not None

    backend_ok = True
    if backend == "kea":
        if has_subnet4:
            dhcp4_enabled = root.find(".//Kea/dhcp4/general/enabled")
            backend_ok &= (dhcp4_enabled is not None and text(dhcp4_enabled) == "1")
        if has_subnet6:
            dhcp6_enabled = root.find(".//Kea/dhcp6/general/enabled")
            backend_ok &= (dhcp6_enabled is not None and text(dhcp6_enabled) == "1")
    else:
        if has_ranges:
            dns_enable = root.find(".//dnsmasq/enable")
            backend_ok &= (dns_enable is not None and text(dns_enable) == "1")

    isc_disabled_v4 = any(text(e) == "" for e in root.findall(".//dhcpd/*/enable"))
    isc_disabled_v6 = any(text(e) == "" for e in root.findall(".//dhcpdv6/*/enable"))

    print(
        f"{base}: backend={backend} expect_enable={has_subnet4 or has_subnet6 or has_ranges} "
        f"backend_enabled_ok={backend_ok} isc_disabled_v4={isc_disabled_v4} isc_disabled_v6={isc_disabled_v6}"
    )
    all_ok = all_ok and backend_ok

print("Backend enabled checks:", "PASS" if all_ok else "FAIL")
PY
```
