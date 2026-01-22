# isc2kea

A safe, production-ready CLI tool to migrate ISC DHCP static mappings to Kea DHCP reservations for both IPv4 and IPv6.

**Designed for OPNsense** config.xml layouts, but may work with similar XML schemas.

**Note**: This tool migrates **static mappings only** (ISC DHCP to Kea reservations) for DHCPv4 and DHCPv6. It does not migrate pools, options, DDNS, PXE, or HA/failover configurations.

**Tested**: Verified against a real OPNsense 25.7.11-generated `config.xml` with Kea DHCPv4/DHCPv6 subnets and ISC static mappings. XML layouts may change in future OPNsense releases; revalidate before using with newer versions.

## TL;DR

1. Create Kea DHCPv4/DHCPv6 subnets in OPNsense first.
2. Take a backup or snapshot
3. Download the config.xml from OPNsense 
4. `isc2kea scan --in ./your-config.xml`
5. `isc2kea convert --in ./your-config.xml --out /conf/config.xml.new`
6. Upload the config.xml back to OPNsense

## Why This Exists

OPNsense is deprecating ISC DHCP in favor of Kea. Static mappings are often the hardest part of that migration, so this open-source tool migrates IPv4/IPv6 static mappings from ISC to Kea using `config.xml`. It does not touch services or reload anything; it only adds reservations to the Kea config.

## Safety First

This tool is designed to be safe on production firewalls:

- **Read-only by default** - No files are modified unless you explicitly use `convert --out`
- **No in-place edits** - Always writes to a separate output file
- **Fails loudly** - Aborts on ambiguity or invalid data (never auto-creates Kea sections)
- **No subnet creation** - Kea subnets must already exist; this tool will not create them
- **No guessing** - Requires exact subnet matches for all IP addresses
- **Duplicate detection** - Handles messy ISC configs with duplicate IPs
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

### Scan (Read-Only Analysis)

Preview what would be migrated without making any changes:

```bash
# On OPNsense (uses default /conf/config.xml)
isc2kea scan

# Or specify path explicitly
isc2kea scan --in /conf/config.xml
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
isc2kea convert --out /tmp/config_migrated.xml

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

Before running, create the required Kea DHCPv4/DHCPv6 subnets in OPNsense. The tool only adds reservations and will error if a matching Kea subnet is missing.

### Abort on Existing Reservations

If you want the tool to fail instead of merging when reservations already exist:

```bash
isc2kea scan --in /conf/config.xml --fail-if-existing
isc2kea convert --in /conf/config.xml --out /tmp/config.xml --fail-if-existing
```

## What Gets Migrated

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

## Example Output

DHCPv4 reservation:

```xml
<reservation uuid="...">
  <subnet>subnet-uuid-v4</subnet>
  <ip_address>10.10.10.101</ip_address>
  <hw_address>08:62:66:27:a9:45</hw_address>
  <hostname>arch</hostname>
</reservation>
```

DHCPv6 reservation:

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

## What Does NOT Get Migrated

- DHCP pools/ranges
- DHCP options
- DDNS settings
- PXE/boot options
- HA/failover configuration

## Conflict Handling

If a Kea reservation already exists with the same IP address, it will be **skipped** (not duplicated). For DHCPv6, existing DUIDs are also treated as duplicates and skipped. The tool reports how many reservations were skipped.

## Error Handling

The tool will **abort** and report an error if:

- An IP address doesn't match any Kea subnet
- The XML is malformed
- Required fields are missing

When multiple subnets overlap, the most specific prefix is selected (largest prefix length).

## Technical Details

- **Language**: Rust
- **XML Handling**: Preserves document structure using xmltree
- **Subnet Matching**: Proper CIDR containment checks using ipnet
- **Kea Lookup**: Recursively searches for `<Kea>`/`<dhcp4>` to support nested configs
- **UUID Generation**: Auto-generates UUIDs for new reservations

## License

BSD 2-Clause License - see LICENSE file for details.

## Support

If this tool saves you time, feel free to buy me a coffee: https://buymeacoffee.com/sheridans
