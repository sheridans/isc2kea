# isc2kea

A safe, production-ready CLI tool to migrate ISC DHCP static mappings to Kea DHCP reservations or dnsmasq host entries for both IPv4 and IPv6.

**Designed for OPNsense** config.xml layouts, but may work with similar XML schemas.

**Note**: This tool migrates **static mappings** (ISC DHCP to Kea reservations or dnsmasq hosts) for DHCPv4 and DHCPv6. It does not migrate DHCP options by default (opt-in subset supported), DDNS, PXE, or HA/failover configurations. Subnet/range creation is optional via `--create-subnets`.

**Tested**: Verified against a real OPNsense 25.7.11-generated `config.xml` with Kea DHCPv4/DHCPv6 subnets and ISC static mappings. XML layouts may change in future OPNsense releases; revalidate before using with newer versions.

## TL;DR

### Migrate to Kea (default)

1. Create Kea DHCPv4/DHCPv6 subnets in OPNsense first (or use `--create-subnets`).
2. Take a backup or snapshot
3. Download the config.xml from OPNsense
4. `isc2kea scan --in ./your-config.xml`
5. `isc2kea convert --in ./your-config.xml --out /conf/config.xml.new`
6. Upload the config.xml back to OPNsense

Optional (create subnets/ranges from ISC if they are not already defined):

```bash
isc2kea scan --in ./your-config.xml --create-subnets
isc2kea convert --in ./your-config.xml --out /conf/config.xml.new --create-subnets
```

Optional (create DHCP options in Kea from ISC):

```bash
isc2kea scan --in ./your-config.xml --create-options
isc2kea convert --in ./your-config.xml --out /conf/config.xml.new --create-options
```

### Migrate to dnsmasq

1. Ensure dnsmasq is configured in `config.xml` (it does not need to be enabled yet).
2. Take a backup or snapshot
3. Download the config.xml from OPNsense
4. `isc2kea scan --in ./your-config.xml --backend dnsmasq`
5. `isc2kea convert --in ./your-config.xml --out /conf/config.xml.new --backend dnsmasq`
6. Upload the config.xml back to OPNsense

Optional (create subnets/ranges from ISC if they are not already defined):

```bash
isc2kea scan --in ./your-config.xml --backend dnsmasq --create-subnets
isc2kea convert --in ./your-config.xml --out /conf/config.xml.new --backend dnsmasq --create-subnets
```

Optional (create DHCP options in dnsmasq from ISC):

```bash
isc2kea scan --in ./your-config.xml --backend dnsmasq --create-options
isc2kea convert --in ./your-config.xml --out /conf/config.xml.new --backend dnsmasq --create-options
```

## Why This Exists

OPNsense is deprecating ISC DHCP in favor of Kea. Static mappings are often the hardest part of that migration, so this open-source tool migrates IPv4/IPv6 static mappings from ISC to Kea or dnsmasq using `config.xml`. It does not touch services or reload anything; it only adds reservations/hosts to the target backend config.

## Safety First

This tool is designed to be safe on production firewalls:

- **Read-only by default** - No files are modified unless you explicitly use `convert --out`
- **No in-place edits** - Always writes to a separate output file
- **Fails loudly** - Aborts on ambiguity or invalid data (never auto-creates backend sections)
- **No subnet/range creation by default** - Subnets/ranges must already exist unless you use `--create-subnets`
- **No DHCP options creation by default** - Kea and dnsmasq options are unchanged unless you use `--create-options`
- **No guessing** - Requires exact subnet matches for all IP addresses (Kea backend)
- **Duplicate detection** - Handles messy ISC configs with duplicate IPs and MACs
- **Case-insensitive** - Works with any tag casing: `<Kea>`/`<kea>`, `<DHCPD>`/`<dhcpd>`, etc.
- **Schema flexible** - Supports standard and alternative Kea plugin XML structures

## Installation

```bash
cargo build --release
```

The binary will be available at `target/release/isc2kea`.

### XML Formatting Note

The output XML will be reformatted (whitespace and indentation may change). This is normal and does not affect OPNsense functionality. The tool preserves all data and structure.

## Usage

**Defaults**:
- `--in` defaults to `/conf/config.xml`
- `--backend` defaults to `kea`

### Optional Subnet/Range Creation (Safe, Opt-In)

By default, the tool **does not create subnets/ranges**. If you want it to create them from ISC ranges:

```bash
isc2kea scan --in /conf/config.xml --create-subnets
isc2kea convert --in /conf/config.xml --out /tmp/config.xml --create-subnets
```

Behavior:
- **Add-only**: existing subnets/ranges are **left untouched** and a warning is printed.
- **Force overwrite** (dangerous): use `--force-subnets` to replace matching subnets/ranges.

Notes:
- Subnets are derived from interface IP + prefix (`<interfaces>`), and pools/ranges come from ISC `<range>` entries.
- **Prefix Delegation (`prefixrange`) is not yet supported** and is ignored.

### Optional DHCP Option Creation (Kea and dnsmasq)

By default, DHCP options are not created. To populate Kea `option_data` or dnsmasq DHCP options from ISC DHCP:

```bash
isc2kea scan --in /conf/config.xml --create-options
isc2kea convert --in /conf/config.xml --out /tmp/config.xml --create-options
```

dnsmasq:

```bash
isc2kea scan --in /conf/config.xml --backend dnsmasq --create-options
isc2kea convert --in /conf/config.xml --out /tmp/config.xml --backend dnsmasq --create-options
```

Behavior:
- **Add-only**: existing Kea/dnsmasq option values are left untouched and a warning is printed.
- **Force overwrite** (dangerous): use `--force-options` to replace existing option values.

Supported (initial set):
- DHCPv4: DNS servers, routers (gateway), domain name, domain search list, NTP servers
- DHCPv6: DNS servers, domain search list
- dnsmasq: `type=set` options only (tag-based `type=match` options are not supported)

dnsmasq option mapping:

| ISC field | dnsmasq option code |
|---|---|
| DHCPv4 `dnsserver` | 6 |
| DHCPv4 `gateway` | 3 |
| DHCPv4 `domain` | 15 |
| DHCPv4 `domainsearchlist` | 119 |
| DHCPv4 `ntpserver` | 42 |
| DHCPv6 `dnsserver` | option6 23 |
| DHCPv6 `domainsearchlist` | option6 24 |

Deferred:
- Static routes / classless static routes
- TFTP / boot options
- Time servers
- Prefix Delegation options

### Backend Selection

Use `--backend` (or `-b`) to choose the target DHCP backend:

```bash
# Kea (default)
isc2kea scan --in /conf/config.xml
isc2kea scan --in /conf/config.xml --backend kea

# dnsmasq
isc2kea scan --in /conf/config.xml --backend dnsmasq
```

### Scan (Read-Only Analysis)

Preview what would be migrated without making any changes:

```bash
# On OPNsense (uses default /conf/config.xml)
isc2kea scan

# Or specify path explicitly
isc2kea scan --in /conf/config.xml

# Scan for dnsmasq migration
isc2kea scan --in /conf/config.xml --backend dnsmasq
```

Output:
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

For detailed output showing each mapping:

```bash
isc2kea scan --in /conf/config.xml --verbose
```

### Convert (Write to New File)

Perform the migration and write to a new file:

```bash
# Kea (default)
isc2kea convert --out /tmp/config_migrated.xml

# dnsmasq
isc2kea convert --out /tmp/config_migrated.xml --backend dnsmasq

# Or specify input explicitly
isc2kea convert --in /conf/config.xml --out /tmp/config_migrated.xml
```

**Safety Features**:
- Tool refuses if output path == input path (prevents config destruction)
- Tool refuses if output file already exists (use `--force` to overwrite)
- Always review the output file before deploying it

**Overwriting output**:
```bash
isc2kea convert --out /tmp/config_migrated.xml --force
```

## Recommended Workflow

1. `isc2kea scan --in /conf/config.xml`
2. `isc2kea convert --in /conf/config.xml --out /conf/config.xml.new`
3. Review the diff between the original and `.new` file
4. Replace the config manually (outside the tool scope)

Before running, create the required Kea DHCPv4/DHCPv6 subnets (or ensure dnsmasq is configured) in OPNsense, or use `--create-subnets`. The tool only adds reservations/hosts and will error if the target backend is not configured.

### Abort on Existing Reservations

If you want the tool to fail instead of merging when reservations already exist:

```bash
isc2kea scan --in /conf/config.xml --fail-if-existing
isc2kea convert --in /conf/config.xml --out /tmp/config.xml --fail-if-existing
```

## What Gets Migrated

### Kea Backend (default)

ISC DHCP static mappings under `<dhcpd>/<interface>/staticmap` are converted to Kea reservations with the following field mapping:

| ISC Field  | Kea Field     | Notes                          |
|------------|---------------|--------------------------------|
| mac        | hw_address    | MAC address                    |
| ipaddr     | ip_address    | IPv4 address                   |
| hostname   | hostname      | Primary hostname               |
| cid        | hostname      | Used if hostname not present   |
| descr      | description   | Description text               |

Each reservation is automatically linked to the correct Kea subnet by matching the IP address against subnet CIDRs.

ISC DHCPv6 static mappings under `<dhcpdv6>/<interface>/staticmap` are converted to Kea DHCPv6 reservations with the following field mapping:

| ISC Field         | Kea Field     | Notes                      |
|-------------------|---------------|----------------------------|
| duid              | duid          | DHCPv6 DUID                |
| ipaddrv6          | ip_address    | IPv6 address               |
| hostname          | hostname      | Primary hostname           |
| descr             | description   | Description text           |
| domainsearchlist  | domain_search | Domain search list         |

Each DHCPv6 reservation is linked to the correct Kea DHCPv6 subnet by matching the IPv6 address against subnet CIDRs.

### dnsmasq Backend

ISC DHCP static mappings are converted to dnsmasq host entries under `<dnsmasq><hosts>`:

| ISC Field  | dnsmasq Field | Notes                          |
|------------|---------------|--------------------------------|
| mac        | hwaddr        | MAC address                    |
| ipaddr     | ip            | IPv4 address                   |
| hostname   | host          | Primary hostname               |
| cid        | client_id     | Client identifier              |
| descr      | descr         | Description text               |

dnsmasq hosts are flat entries (no subnet association required). IPv6 mappings are supported via `client_id` (DUID) when present.

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

## What Does NOT Get Migrated

- DHCP pools/ranges (unless `--create-subnets` is used)
- DHCP options (unless `--create-options` is used for Kea or dnsmasq)
- DDNS settings
- PXE/boot options
- HA/failover configuration

## Conflict Handling

**Kea**: If a reservation already exists with the same IP address, it will be **skipped** (not duplicated). For DHCPv6, existing DUIDs are also treated as duplicates and skipped.

**dnsmasq**: If a host entry already exists with the same IP address, MAC address, or client_id (DUID), it will be **skipped**.

The tool reports how many entries were skipped.

## Error Handling

The tool will **abort** and report an error if:

- An IP address doesn't match any Kea subnet (Kea backend)
- The target backend is not configured in config.xml
- The XML is malformed
- Required fields are missing

When multiple subnets overlap (Kea), the most specific prefix is selected (largest prefix length).

## Technical Details

- **Language**: Rust
- **XML Handling**: Preserves document structure using xmltree
- **Subnet Matching**: Proper CIDR containment checks using ipnet (Kea backend)
- **Backend Dispatch**: Enum-based dispatch for clean separation of Kea and dnsmasq logic
- **UUID Generation**: Auto-generates UUIDs for new reservations/hosts

## License

BSD 2-Clause License - see LICENSE file for details.

## Support

If this tool saves you time, feel free to buy me a coffee: https://buymeacoffee.com/sheridans
