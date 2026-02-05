use anyhow::{Context, Result};
use std::io::{Read, Write};
use xmltree::{Element, EmitterConfig};

use crate::backend::Backend;
use crate::extract::{
    extract_isc_mappings, extract_isc_mappings_v6, extract_isc_ranges, extract_isc_ranges_v6,
    extract_kea_subnets, extract_kea_subnets_v6,
};
use crate::{MigrationOptions, MigrationStats};

mod dnsmasq;
mod kea;
mod options;
pub(crate) mod services;
mod subnets;
mod utils;

/// Scan the configuration and return basic counts without validation
pub fn scan_counts<R: Read>(reader: R, backend: &Backend) -> Result<MigrationStats> {
    let root = Element::parse(reader).context("Failed to parse XML")?;

    let isc_mappings = extract_isc_mappings(&root)?;
    let isc_mappings_v6 = extract_isc_mappings_v6(&root)?;
    let isc_ranges = extract_isc_ranges(&root)?;
    let isc_ranges_v6 = extract_isc_ranges_v6(&root)?;

    let (target_subnets_found, target_subnets_v6_found) = match backend {
        Backend::Kea => {
            let kea_subnets = extract_kea_subnets(&root)?;
            let kea_subnets_v6 = extract_kea_subnets_v6(&root)?;
            (kea_subnets.len(), kea_subnets_v6.len())
        }
        Backend::Dnsmasq => (0, 0),
    };

    Ok(MigrationStats {
        isc_mappings_found: isc_mappings.len(),
        isc_mappings_v6_found: isc_mappings_v6.len(),
        isc_ranges_found: isc_ranges.len(),
        isc_ranges_v6_found: isc_ranges_v6.len(),
        target_subnets_found,
        target_subnets_v6_found,
        reservations_to_create: 0,
        reservations_v6_to_create: 0,
        reservations_skipped: 0,
        reservations_v6_skipped: 0,
        ..Default::default()
    })
}

/// Scan the configuration and return statistics without modifying anything
pub fn scan_config<R: Read>(reader: R, options: &MigrationOptions) -> Result<MigrationStats> {
    let root = Element::parse(reader).context("Failed to parse XML")?;
    let isc_mappings = extract_isc_mappings(&root)?;
    let isc_mappings_v6 = extract_isc_mappings_v6(&root)?;
    let isc_ranges = extract_isc_ranges(&root)?;
    let isc_ranges_v6 = extract_isc_ranges_v6(&root)?;

    let mut stats = match options.backend {
        Backend::Kea => kea::scan_kea(&root, &isc_mappings, &isc_mappings_v6, options),
        Backend::Dnsmasq => dnsmasq::scan_dnsmasq(&root, &isc_mappings, &isc_mappings_v6, options),
    }?;

    stats.isc_ranges_found = isc_ranges.len();
    stats.isc_ranges_v6_found = isc_ranges_v6.len();

    Ok(stats)
}

/// Convert ISC static mappings into the target backend format, writing the
/// updated XML and reporting migration stats.
pub fn convert_config<R: Read, W: Write>(
    reader: R,
    writer: W,
    options: &MigrationOptions,
) -> Result<MigrationStats> {
    let mut root = Element::parse(reader).context("Failed to parse XML")?;
    let isc_mappings = extract_isc_mappings(&root)?;
    let isc_mappings_v6 = extract_isc_mappings_v6(&root)?;

    let stats = match options.backend {
        Backend::Kea => kea::convert_kea(&mut root, &isc_mappings, &isc_mappings_v6, options)?,
        Backend::Dnsmasq => {
            dnsmasq::convert_dnsmasq(&mut root, &isc_mappings, &isc_mappings_v6, options)?
        }
    };

    // Write the updated XML with human-readable indentation
    let emitter_config = EmitterConfig::new()
        .perform_indent(true)
        .indent_string("  ")
        .write_document_declaration(true);
    root.write_with_config(writer, emitter_config)
        .context("Failed to write XML")?;

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fail_if_existing_flag() {
        let xml_with_existing = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <lan>
            <ipaddr>192.168.1.1</ipaddr>
            <subnet>24</subnet>
        </lan>
    </interfaces>
    <dhcpd>
        <lan>
            <staticmap>
                <mac>00:11:22:33:44:55</mac>
                <ipaddr>192.168.1.10</ipaddr>
                <hostname>testhost</hostname>
            </staticmap>
        </lan>
    </dhcpd>
    <Kea>
        <dhcp4>
            <subnets>
                <subnet4 uuid="test-subnet-uuid-1234">
                    <subnet>192.168.1.0/24</subnet>
                </subnet4>
            </subnets>
            <reservations>
                <reservation uuid="existing-reservation">
                    <ip_address>192.168.1.99</ip_address>
                    <hw_address>99:99:99:99:99:99</hw_address>
                </reservation>
            </reservations>
        </dhcp4>
    </Kea>
</opnsense>
"#;

        let input = std::io::Cursor::new(xml_with_existing);
        let options = MigrationOptions {
            fail_if_existing: true,
            verbose: false,
            ..Default::default()
        };

        let result = scan_config(input, &options);
        assert!(
            result.is_err(),
            "Should fail when existing reservations found with --fail-if-existing"
        );
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Existing reservations found"));
    }

    #[test]
    fn test_scan_counts_kea_and_dnsmasq() {
        let xml = r#"<?xml version="1.0"?>
<opnsense>
  <dhcpd>
    <lan>
      <staticmap>
        <mac>00:11:22:33:44:55</mac>
        <ipaddr>192.168.1.10</ipaddr>
      </staticmap>
    </lan>
  </dhcpd>
  <Kea>
    <dhcp4>
      <subnets>
        <subnet4 uuid="s1">
          <subnet>192.168.1.0/24</subnet>
        </subnet4>
      </subnets>
    </dhcp4>
    <dhcp6>
      <subnets>
        <subnet6 uuid="s2">
          <subnet>2001:db8::/64</subnet>
        </subnet6>
      </subnets>
    </dhcp6>
  </Kea>
</opnsense>
"#;

        let stats_kea = scan_counts(std::io::Cursor::new(xml.as_bytes()), &Backend::Kea).unwrap();
        assert_eq!(stats_kea.isc_mappings_found, 1);
        assert_eq!(stats_kea.target_subnets_found, 1);
        assert_eq!(stats_kea.target_subnets_v6_found, 1);

        let stats_dns =
            scan_counts(std::io::Cursor::new(xml.as_bytes()), &Backend::Dnsmasq).unwrap();
        assert_eq!(stats_dns.target_subnets_found, 0);
        assert_eq!(stats_dns.target_subnets_v6_found, 0);
    }
}
