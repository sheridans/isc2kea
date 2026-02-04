# isc2kea

An OPNsense ISC DHCP to Kea/dnsmasq migration tool for DHCPv4 and DHCPv6.

OPNsense is deprecating ISC DHCP (isc-dhcp) in favor of Kea. This tool reads your `config.xml` and migrates your ISC DHCP configuration (static mappings, subnets, ranges, and DHCP options) into Kea reservations or dnsmasq host entries. It handles both IPv4 and IPv6, and can:

- **Migrate fixed IP assignments** (ISC static mappings to Kea reservations or dnsmasq hosts)
- **Create subnets and ranges** in the target backend from your ISC config (`--create-subnets`)
- **Copy DHCP options** like DNS servers, gateway, domain, and NTP (`--create-options`)
- **Validate interfaces** to catch misconfigurations before they reach production

**It does not touch your running system.** It only reads and writes config files.

## Quick Start

### Migrate to Kea (default)

1. **Take a snapshot or backup** of your OPNsense box before making changes.
2. Download `config.xml` from the OPNsense UI.
3. Kea does not need to be enabled yet. You can review the migrated config before disabling ISC DHCP and enabling Kea.

Add `--create-subnets` to create Kea subnets from your ISC config (or create them in OPNsense first). Add `--create-options` to also copy DHCP options (DNS, gateway, NTP, etc.). Drop either flag if you don't need it.

```bash
# 1. Preview what would be migrated (no changes made)
isc2kea scan --in ./config.xml --create-subnets --create-options

# 2. Perform the migration
isc2kea convert --in ./config.xml --out ./config.xml.new --create-subnets --create-options
```

### Migrate to dnsmasq

1. **Take a snapshot or backup** of your OPNsense box before making changes.
2. Download `config.xml` from the OPNsense UI.
3. Make sure `<dnsmasq>` exists in your `config.xml`. It does not need to be enabled yet. You can review the migrated config before disabling ISC DHCP and enabling dnsmasq.

Add `--create-subnets` to create dnsmasq ranges from your ISC config (or create them in OPNsense first). Add `--create-options` to also copy DHCP options. Drop either flag if you don't need it.

```bash
# 1. Preview
isc2kea scan --in ./config.xml --backend dnsmasq --create-subnets --create-options

# 2. Convert
isc2kea convert --in ./config.xml --out ./config.xml.new --backend dnsmasq --create-subnets --create-options
```

### Then what?

1. Review the new file and compare it to your original
2. Upload `config.xml.new` via the OPNsense UI (replacing the original)
3. Disable ISC DHCP and enable Kea (or dnsmasq) when you're happy with the config
4. Reboot or reload the DHCP service
5. Leases will appear after clients renew (you may need to renew or reboot clients)

**Important: select your DHCP interfaces before enabling the new backend.** The tool does not configure which interfaces Kea or dnsmasq listens on. Without this, DHCP will not serve any clients.

- **Kea**: Services > Kea DHCP > Settings > Interfaces
- **dnsmasq**: Services > Dnsmasq DNS > Settings > Interfaces

**Note: leases are not migrated.** The tool converts configuration only. Existing DHCP leases from ISC DHCP will not carry over — clients will request new leases from the new backend.

## Installation

Download the latest binary for your platform from [GitHub Releases](https://github.com/sheridans/isc2kea/releases):

| Platform | File |
|----------|------|
| FreeBSD x86_64 (OPNsense) | `isc2kea-freebsd-x86_64.tar.gz` |
| Linux x86_64 | `isc2kea-linux-x86_64.tar.gz` |
| Linux aarch64 | `isc2kea-linux-aarch64.tar.gz` |
| macOS x86_64 | `isc2kea-macos-x86_64.tar.gz` |
| Windows x86_64 | `isc2kea-windows-x86_64.zip` |

Or build from source (requires [Rust](https://rustup.rs/)):

```bash
cargo build --release
```

## Usage

### Commands

| Command | What it does |
|---------|-------------|
| `scan` | Read-only preview. Shows what would be migrated without changing anything. |
| `convert` | Performs the migration and writes the result to a new file. |

### Flags

| Flag | Description |
|------|-------------|
| `--in <path>` | Input config file. Defaults to `/conf/config.xml`. |
| `--out <path>` | Output file (convert only). Must be different from input. |
| `--backend <kea\|dnsmasq>` | Target DHCP backend. Defaults to `kea`. |
| `--create-subnets` | Create subnets/ranges in the target backend from your ISC config. Without this, subnets must already exist. |
| `--force-subnets` | Overwrite existing subnets/ranges (use with `--create-subnets`). |
| `--create-options` | Copy DHCP options (DNS servers, gateway, etc.) from ISC to the target backend. |
| `--force-options` | Overwrite existing DHCP options (use with `--create-options`). |
| `--fail-if-existing` | Abort if any reservations/hosts already exist in the target backend. |
| `--force` | Overwrite the output file if it already exists (convert only). |
| `--verbose` | Show details for each individual mapping. |

### Automatic Subnet/Range Creation (`--create-subnets`)

By default, Kea subnets or dnsmasq ranges must already exist in your config before migrating. If they don't, add `--create-subnets` and the tool will create them for you based on your existing ISC DHCP config:

- **Subnets** are built from each network interface's IP address and prefix length (from `<interfaces>` in your config).
- **Pools/ranges** are copied from your ISC DHCP `<range>` entries.
- Existing subnets are left alone. New ones are only added if they don't already exist. Use `--force-subnets` to replace existing ones instead.

```bash
isc2kea scan --in ./config.xml --create-subnets
isc2kea convert --in ./config.xml --out ./config.xml.new --create-subnets
```

### Copying DHCP Options (`--create-options`)

By default, DHCP options (DNS servers, gateway, domain, etc.) are not touched. Add `--create-options` to copy them from ISC DHCP into the target backend:

- Existing option values are left alone. Only missing values are filled in. Use `--force-options` to overwrite them instead.
- **Kea**: options are attached to subnets, so `--create-options` requires Kea subnets to exist. If they don't, combine with `--create-subnets` to create them in the same run.
- **dnsmasq**: options are independent of ranges and will be created regardless.

```bash
isc2kea scan --in ./config.xml --create-options
isc2kea convert --in ./config.xml --out ./config.xml.new --create-options
```

Both flags can be combined:

```bash
isc2kea convert --in ./config.xml --out ./config.xml.new --backend dnsmasq --create-subnets --create-options
```

### Interface Validation

The tool checks that each fixed IP assignment actually belongs to the correct network interface. For example, if a device with IP `10.0.0.50` is listed under your `lan` interface but `lan` is a `192.168.1.0/24` network, the tool will abort and tell you exactly which entry has the mismatch. This prevents devices from being silently assigned to the wrong subnet.

### Sample output (scan)

```
ISC DHCP static mappings found: 45
ISC DHCPv6 static mappings found: 12
Kea subnet4 entries found: 3
Kea subnet6 entries found: 2
Reservations that would be created: 43
Reservations (v6) that would be created: 10
Reservations skipped (already exist): 2
Reservations skipped (v6): 2
```

## What Gets Migrated

### To Kea

Your ISC DHCP fixed assignments become Kea reservations. Each one is automatically matched to the correct Kea subnet based on its IP address.

**IPv4:**

| ISC field | Kea field | What it is |
|-----------|-----------|------------|
| mac | hw_address | Device MAC address |
| ipaddr | ip_address | Fixed IPv4 address |
| hostname | hostname | Device name |
| cid | hostname | Used as hostname if hostname is empty |
| descr | description | Description |

**IPv6:**

| ISC field | Kea field | What it is |
|-----------|-----------|------------|
| duid | duid | Device unique ID (DHCPv6) |
| ipaddrv6 | ip_address | Fixed IPv6 address |
| hostname | hostname | Device name |
| descr | description | Description |
| domainsearchlist | domain_search | DNS search domains |

### To dnsmasq

Your ISC DHCP fixed assignments become dnsmasq host entries. No subnet matching is needed since dnsmasq uses a flat list.

| ISC field | dnsmasq field | What it is |
|-----------|---------------|------------|
| mac | hwaddr | Device MAC address |
| ipaddr | ip | Fixed IP address |
| hostname | host | Device name |
| cid | client_id | Client identifier |
| descr | descr | Description |

IPv6 entries are also supported when a DUID is present.

### DHCP Options (with `--create-options`)

The following DHCP options can be copied from ISC to Kea or dnsmasq:

**IPv4:** DNS servers, gateway, domain name, domain search list, NTP servers

**IPv6:** DNS servers, domain search list

dnsmasq option mapping:

| ISC field | dnsmasq option code |
|-----------|---------------------|
| DHCPv4 `dnsserver` | 6 |
| DHCPv4 `gateway` | 3 |
| DHCPv4 `domain` | 15 |
| DHCPv4 `domainsearchlist` | 119 |
| DHCPv4 `ntpserver` | 42 |
| DHCPv6 `dnsserver` | option6 23 |
| DHCPv6 `domainsearchlist` | option6 24 |


## How It Handles Conflicts

- **Duplicates are skipped.** If a reservation or host already exists with the same IP, MAC, or DUID, it won't be duplicated. The tool tells you how many were skipped.
- **Subnets are add-only.** With `--create-subnets`, existing subnets are left alone (unless you also use `--force-subnets`).
- **Options are add-only.** With `--create-options`, existing option values are left alone (unless you also use `--force-options`).

## Safety

- **Nothing changes unless you run `convert --out`**. `scan` is always read-only.
- **Never overwrites your input**. Refuses to write to the same file you read from.
- **Never overwrites existing output**. Refuses if the output file exists (unless you use `--force`).
- **Interface validation**. Checks that each device's IP actually belongs to the network interface it's listed under. Aborts if there's a mismatch, so you never accidentally put a device on the wrong subnet.
- **Validates everything**. Checks that IPs match subnets and that the target backend is actually configured. Aborts on any problem.
- **Works with messy configs**. Handles duplicate entries, mixed tag casing (`<Kea>`/`<kea>`), and different Kea plugin XML structures.

## Limitations

**Not migrated (out of scope):**
- DDNS settings
- PXE/boot options
- HA/failover configuration

**Not yet supported:**
- Prefix delegation (`prefixrange`) is ignored during subnet creation
- dnsmasq `type=match` DHCP options are not migrated. Only `type=set` options are supported. OPNsense uses `match`/`set` pairs for tag-based option assignment; only the `set` (value) side is handled.
- DHCP options: static routes, classless static routes, TFTP/boot, and time servers
- IPv6 interfaces using `track6` or `dhcp6` addressing are skipped (no static CIDR to derive)
- ISC entries missing required fields (e.g. no MAC or no IP) are silently skipped

**Opt-in only (not migrated by default):**
- DHCP pools/ranges (use `--create-subnets`)
- DHCP options (use `--create-options`)

## Example Output

### Kea DHCPv4 reservation

```xml
<reservation uuid="...">
  <subnet>subnet-uuid-v4</subnet>
  <ip_address>10.10.10.101</ip_address>
  <hw_address>08:62:66:27:a9:45</hw_address>
  <hostname>arch</hostname>
</reservation>
```

### Kea DHCPv6 reservation

```xml
<reservation uuid="...">
  <subnet>subnet-uuid-v6</subnet>
  <ip_address>2001:db8:42::10</ip_address>
  <duid>00:01:00:01:aa:bb:cc:dd:00:11:22:33:44:55</duid>
  <hostname>host1</hostname>
  <domain_search>mydomain.local</domain_search>
  <description>test device 1</description>
</reservation>
```

### dnsmasq host entry

```xml
<hosts uuid="...">
  <hwaddr>08:62:66:27:a9:45</hwaddr>
  <ip>10.10.10.101</ip>
  <host>arch</host>
  <descr>my workstation</descr>
  <domain></domain>
  <local>0</local>
  <ignore>0</ignore>
</hosts>
```

## Notes

- The output XML may have different whitespace/indentation than the original. This is cosmetic and does not affect OPNsense.
- When multiple subnets overlap, the most specific one (longest prefix) is used.
- Tested against real OPNsense `config.xml` files from 25.7 and 26.1. An example config from a live OPNsense 25.7.11 system is included in `example/live.xml`.
- XML layouts may change in future OPNsense releases; revalidate before using with newer versions.

## License

BSD 2-Clause License - see LICENSE file for details.

## Support

If this tool saves you time, feel free to buy me a coffee:

[![Buy Me A Coffee](https://img.shields.io/badge/Buy%20Me%20A%20Coffee-support-yellow?style=flat&logo=buy-me-a-coffee)](https://buymeacoffee.com/sheridans)
