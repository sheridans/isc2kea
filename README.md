# isc2kea

![CI](https://github.com/sheridans/isc2kea/actions/workflows/ci.yml/badge.svg)
![Release](https://img.shields.io/github/v/release/sheridans/isc2kea?display_name=tag)

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

Add `--create-subnets` to create Kea subnets from your ISC config (or create them in OPNsense first). Add `--create-options` to copy DHCP options (DNS, gateway, NTP, etc.). Drop either flag if you don't need it.

```bash
# 1. Preview what would be migrated (no changes made)
isc2kea scan --in ./config.xml --create-subnets --create-options

# 2. Perform the migration
isc2kea convert --in ./config.xml --out ./config.xml.new --create-subnets --create-options
```
Review `config.xml.new` before disabling ISC DHCP and enabling Kea. When you're ready, use `--enable-backend` to automate the switch.

### Migrate to dnsmasq

1. **Take a snapshot or backup** of your OPNsense box before making changes.
2. Download `config.xml` from the OPNsense UI.
3. Make sure `<dnsmasq>` exists in your `config.xml`. It does not need to be enabled yet. You can review the migrated config before disabling ISC DHCP and enabling dnsmasq.

Add `--create-subnets` to create dnsmasq ranges from your ISC config (or create them in OPNsense first). Add `--create-options` to copy DHCP options. Drop either flag if you don't need it.

```bash
# 1. Preview
isc2kea scan --in ./config.xml --backend dnsmasq --create-subnets --create-options

# 2. Convert
isc2kea convert --in ./config.xml --out ./config.xml.new --backend dnsmasq --create-subnets --create-options
```
Review `config.xml.new` before disabling ISC DHCP and enabling dnsmasq. When you're ready, use `--enable-backend` to automate the switch.

### Then what?

1. Review the new file and compare it to your original
2. Upload `config.xml.new` via the OPNsense UI (replacing the original)
3. Disable ISC DHCP and enable Kea (or dnsmasq) when you're happy with the config (or use `--enable-backend`)
4. Reboot or reload the DHCP service
5. Leases will appear after clients renew (you may need to renew or reboot clients)

**Tip:** Use `--enable-backend` to automatically disable ISC DHCP on interfaces enabled in your ISC config and enable the target backend. If you also use `--create-subnets`, the backend listening interfaces are configured automatically — no manual UI steps needed.

**Note:** `--enable-backend` is intended for the initial cutover from ISC. If ISC is already disabled, the tool will refuse to enable another backend to avoid dual‑DHCP. For repeat runs, omit `--enable-backend` and manage backend switches manually.

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
| `verify` | Show a diff of what would change without writing any files (exit code 1 if changes). |

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
| `--enable-backend` | Disable ISC DHCP on interfaces enabled in the ISC config and enable the target backend (convert only). |
| `--force` | Overwrite the output file if it already exists (convert only). |
| `--verbose` | Show details for each individual mapping. |

### Automatic Subnet/Range Creation (`--create-subnets`)

By default, Kea subnets or dnsmasq ranges must already exist in your config before migrating. If they don't, add `--create-subnets` and the tool will create them for you based on your existing ISC DHCP config:

- **Subnets** are built from each network interface's IP address and prefix length (from `<interfaces>` in your config).
- **Pools/ranges** are copied from your ISC DHCP `<range>` entries.
- **Interfaces** are automatically configured so the backend listens on the correct networks.
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

### Workflows / Cookbook

Common workflows you can copy/paste:

1. **Preview only (no changes)**

```bash
isc2kea scan --in ./config.xml
```

2. **Verify (diff only, no files written)**

```bash
isc2kea verify --in ./config.xml --create-subnets --create-options
```
Use `--quiet` to suppress diff output; exit code is 1 if changes are detected.

Example output (trimmed):

```diff
--- original
+++ converted
@@ -374,7 +374,7 @@
   <dhcpd>
     <lan>
-      <enable>1</enable>
+      <enable />
@@ -1430,19 +1430,38 @@
     <Kea>
       <dhcp4 persisted_at="..." version="...">
         <general>
-          <enabled>0</enabled>
+          <enabled>1</enabled>
@@
-          <interfaces />
+          <interfaces>lan,opt1</interfaces>
@@
-        <subnets />
+        <subnets>
+          <subnet4 uuid="...">
+            <subnet>192.168.69.0/24</subnet>
+            <pools>192.168.69.100-192.168.69.200</pools>
+            <option_data>...</option_data>
+          </subnet4>
+        </subnets>
```

3. **Convert and review (no backend enable)**

```bash
isc2kea convert --in ./config.xml --out ./config.xml.new --create-subnets --create-options
```

4. **Convert and enable Kea (ISC → Kea)**

```bash
isc2kea convert --in ./config.xml --out ./config.xml.new \
  --backend kea \
  --create-subnets \
  --create-options \
  --enable-backend
```

5. **Convert and enable dnsmasq (ISC → dnsmasq)**

```bash
isc2kea convert --in ./config.xml --out ./config.xml.new \
  --backend dnsmasq \
  --create-subnets \
  --create-options \
  --enable-backend
```

6. **Create subnets/ranges only**

```bash
isc2kea convert --in ./config.xml --out ./config.xml.new --create-subnets
```

7. **Create DHCP options only**

```bash
isc2kea convert --in ./config.xml --out ./config.xml.new --create-options
```

8. **Force replace existing subnets/options**

```bash
isc2kea convert --in ./config.xml --out ./config.xml.new \
  --create-subnets --force-subnets \
  --create-options --force-options
```

### Scripted Usage

The tool is non-interactive and can be used in scripts to migrate multiple firewalls:

```bash
#!/usr/bin/env bash
set -euo pipefail
FIREWALLS="fw1.example.com fw2.example.com fw3.example.com"
ISC2KEA="isc2kea"
BACKEND="kea" # or "dnsmasq"
CONVERT_ARGS=(--backend "$BACKEND" --create-subnets --create-options --enable-backend --force)
VERIFY_ARGS=(--backend "$BACKEND" --create-subnets --create-options --quiet)

for fw in $FIREWALLS; do
    echo "[$fw] Migrating..."

    # Download config
    echo "[$fw] Downloading config..."
    scp root@$fw:/conf/config.xml /tmp/${fw}-config.xml

    # Convert (--force overwrites existing output file)
    echo "[$fw] Converting config..."
    "$ISC2KEA" convert \
        --in /tmp/${fw}-config.xml \
        --out /tmp/${fw}-config.xml.new \
        "${CONVERT_ARGS[@]}"

    # Verify converted config is stable (idempotent) before touching the firewall
    echo "[$fw] Verifying converted config..."
    if ! "$ISC2KEA" verify \
        --in /tmp/${fw}-config.xml.new \
        "${VERIFY_ARGS[@]}"; then
        echo "[$fw] Verify failed; skipping upload/reboot"
        continue
    fi

    # Upload modified config
    echo "[$fw] Uploading config..."
    scp /tmp/${fw}-config.xml.new root@$fw:/conf/config.xml

    # Restart services in a safe order: stop ISC first, then start target backend
    if [ "$BACKEND" = "kea" ]; then
        echo "[$fw] Restarting services: dhcpd, dhcpd6, kea..."
        ssh root@$fw "configctl dhcpd restart && configctl dhcpd6 restart && configctl kea restart"
    else
        echo "[$fw] Restarting services: dhcpd, dhcpd6, dnsmasq..."
        ssh root@$fw "configctl dhcpd restart && configctl dhcpd6 restart && configctl dnsmasq restart"
    fi
done
```

You can also use the OPNsense API to download and upload configs instead of SCP.

### Sample output (scan)

```
ISC DHCP static mappings found: 45
ISC DHCPv6 static mappings found: 12
ISC DHCP ranges found: 3
ISC DHCPv6 ranges found: 2
Kea subnet4 entries found: 3
Kea subnet6 entries found: 2
Reservations that would be created: 43
Reservations (v6) that would be created: 10
Reservations skipped (already exist): 2
Reservations skipped (v6): 2
```

With `--verbose`, scan also prints enabled ISC interfaces:

```
ISC DHCP enabled interfaces (v4): lan, opt1, opt2
ISC DHCP enabled interfaces (v6): lan
```

### Sample output (convert)

```
Migration completed successfully!
ISC DHCP static mappings found: 45
ISC DHCPv6 static mappings found: 12
Kea subnet4 entries found: 3
Kea subnet6 entries found: 2
Reservations created: 43
Reservations created (v6): 10
Reservations skipped (already exist): 2
Reservations skipped (v6): 2
Interfaces configured: lan, opt1, opt2
ISC DHCP disabled (v4): lan, opt1, opt2
Backend DHCP enabled (v4): yes
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
- Tested against real OPNsense `config.xml` files from 25.7 and 26.1.
- XML layouts may change in future OPNsense releases; revalidate before using with newer versions.

## License

BSD 2-Clause License - see LICENSE file for details.

## Support

If this tool saves you time, feel free to buy me a coffee:

[![Buy Me A Coffee](https://img.shields.io/badge/Buy%20Me%20A%20Coffee-support-yellow?style=flat&logo=buy-me-a-coffee)](https://buymeacoffee.com/sheridans)
