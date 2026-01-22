use anyhow::{anyhow, Context, Result};
use std::io::{Read, Write};
use xmltree::{Element, EmitterConfig, XMLNode};

use crate::extract::{
    extract_existing_reservation_duids_v6, extract_existing_reservation_ips,
    extract_existing_reservation_ips_v6, extract_isc_mappings, extract_isc_mappings_v6,
    extract_kea_subnets, extract_kea_subnets_v6,
};
use crate::migrate_v4::{create_reservation_element, get_reservations_node};
use crate::migrate_v6::{create_reservation_element_v6, get_reservations_node_v6};
use crate::subnet::{find_subnet_for_ip, find_subnet_for_ip_v6};
use crate::xml_helpers::{has_kea_dhcp4, has_kea_dhcp6};
use crate::{MigrationError, MigrationOptions, MigrationStats};

fn short_uuid(uuid: &str) -> &str {
    uuid.get(..8).unwrap_or(uuid)
}

/// Scan the configuration and return basic counts without validation
pub fn scan_counts<R: Read>(reader: R) -> Result<MigrationStats> {
    let root = Element::parse(reader).context("Failed to parse XML")?;

    let isc_mappings = extract_isc_mappings(&root)?;
    let kea_subnets = extract_kea_subnets(&root)?;
    let isc_mappings_v6 = extract_isc_mappings_v6(&root)?;
    let kea_subnets_v6 = extract_kea_subnets_v6(&root)?;

    Ok(MigrationStats {
        isc_mappings_found: isc_mappings.len(),
        isc_mappings_v6_found: isc_mappings_v6.len(),
        kea_subnets_found: kea_subnets.len(),
        kea_subnets_v6_found: kea_subnets_v6.len(),
        reservations_to_create: 0,
        reservations_v6_to_create: 0,
        reservations_skipped: 0,
        reservations_v6_skipped: 0,
    })
}

/// Scan the configuration and return statistics without modifying anything
pub fn scan_config<R: Read>(reader: R, options: &MigrationOptions) -> Result<MigrationStats> {
    let root = Element::parse(reader).context("Failed to parse XML")?;

    let isc_mappings = extract_isc_mappings(&root)?;
    let kea_subnets = extract_kea_subnets(&root)?;
    let existing_ips = extract_existing_reservation_ips(&root)?;
    let isc_mappings_v6 = extract_isc_mappings_v6(&root)?;
    let kea_subnets_v6 = extract_kea_subnets_v6(&root)?;
    let existing_ips_v6 = extract_existing_reservation_ips_v6(&root)?;
    let existing_duids_v6 = extract_existing_reservation_duids_v6(&root)?;

    // Early check: differentiate between "Kea not configured" vs "no subnets"
    if !isc_mappings.is_empty() && kea_subnets.is_empty() {
        if !has_kea_dhcp4(&root) {
            return Err(MigrationError::KeaNotConfigured.into());
        }
        return Err(MigrationError::NoKeaSubnets.into());
    }

    if !isc_mappings_v6.is_empty() && kea_subnets_v6.is_empty() {
        if !has_kea_dhcp6(&root) {
            return Err(MigrationError::KeaV6NotConfigured.into());
        }
        return Err(MigrationError::NoKeaSubnetsV6.into());
    }

    // Check fail_if_existing flag
    if options.fail_if_existing
        && (!existing_ips.is_empty()
            || !existing_ips_v6.is_empty()
            || !existing_duids_v6.is_empty())
    {
        return Err(anyhow!(
            "Existing reservations found ({} IPs) and --fail-if-existing is set. Aborting.",
            existing_ips.len() + existing_ips_v6.len()
        ));
    }

    let mut to_create = 0;
    let mut skipped = 0;
    let mut to_create_v6 = 0;
    let mut skipped_v6 = 0;

    // Track reserved IPs including ones we're planning to add (to catch ISC duplicates)
    let mut reserved_ips = existing_ips;
    let mut reserved_ips_v6 = existing_ips_v6;
    let mut reserved_duids_v6 = existing_duids_v6;

    if options.verbose {
        println!("\nProcessing {} ISC static mappings:", isc_mappings.len());
        if !isc_mappings_v6.is_empty() {
            println!(
                "Processing {} ISC DHCPv6 static mappings:",
                isc_mappings_v6.len()
            );
        }
    }

    for mapping in &isc_mappings {
        if reserved_ips.contains(&mapping.ipaddr) {
            skipped += 1;
            if options.verbose {
                println!(
                    "  SKIP: {} ({}) - IP already reserved",
                    mapping.ipaddr, mapping.mac
                );
            }
        } else {
            // Check if we can find a subnet (this validates the migration would succeed)
            let subnet_uuid = find_subnet_for_ip(&mapping.ipaddr, &kea_subnets)?;
            reserved_ips.insert(mapping.ipaddr.clone());
            to_create += 1;
            if options.verbose {
                let hostname = mapping
                    .hostname
                    .as_ref()
                    .or(mapping.cid.as_ref())
                    .map(|s| s.as_str())
                    .unwrap_or("<no hostname>");
                println!(
                    "  ADD: {} ({}) -> subnet {} [{}]",
                    mapping.ipaddr,
                    mapping.mac,
                    short_uuid(&subnet_uuid),
                    hostname
                );
            }
        }
    }

    for mapping in &isc_mappings_v6 {
        if reserved_ips_v6.contains(&mapping.ipaddr) || reserved_duids_v6.contains(&mapping.duid) {
            skipped_v6 += 1;
            if options.verbose {
                println!(
                    "  SKIP6: {} ({}) - IP or DUID already reserved",
                    mapping.ipaddr, mapping.duid
                );
            }
        } else {
            let subnet_uuid = find_subnet_for_ip_v6(&mapping.ipaddr, &kea_subnets_v6)?;
            reserved_ips_v6.insert(mapping.ipaddr.clone());
            reserved_duids_v6.insert(mapping.duid.clone());
            to_create_v6 += 1;
            if options.verbose {
                let hostname = mapping.hostname.as_deref().unwrap_or("<no hostname>");
                println!(
                    "  ADD6: {} ({}) -> subnet {} [{}]",
                    mapping.ipaddr,
                    mapping.duid,
                    short_uuid(&subnet_uuid),
                    hostname
                );
            }
        }
    }

    Ok(MigrationStats {
        isc_mappings_found: isc_mappings.len(),
        isc_mappings_v6_found: isc_mappings_v6.len(),
        kea_subnets_found: kea_subnets.len(),
        kea_subnets_v6_found: kea_subnets_v6.len(),
        reservations_to_create: to_create,
        reservations_v6_to_create: to_create_v6,
        reservations_skipped: skipped,
        reservations_v6_skipped: skipped_v6,
    })
}

/// Convert ISC DHCPv4/DHCPv6 static mappings into Kea reservations, writing the
/// updated XML and reporting migration stats.
pub fn convert_config<R: Read, W: Write>(
    reader: R,
    writer: W,
    options: &MigrationOptions,
) -> Result<MigrationStats> {
    let mut root = Element::parse(reader).context("Failed to parse XML")?;

    let isc_mappings = extract_isc_mappings(&root)?;
    let kea_subnets = extract_kea_subnets(&root)?;
    let existing_ips = extract_existing_reservation_ips(&root)?;
    let isc_mappings_v6 = extract_isc_mappings_v6(&root)?;
    let kea_subnets_v6 = extract_kea_subnets_v6(&root)?;
    let existing_ips_v6 = extract_existing_reservation_ips_v6(&root)?;
    let existing_duids_v6 = extract_existing_reservation_duids_v6(&root)?;

    // Early check: differentiate between "Kea not configured" vs "no subnets"
    if !isc_mappings.is_empty() && kea_subnets.is_empty() {
        if !has_kea_dhcp4(&root) {
            return Err(MigrationError::KeaNotConfigured.into());
        }
        return Err(MigrationError::NoKeaSubnets.into());
    }

    if !isc_mappings_v6.is_empty() && kea_subnets_v6.is_empty() {
        if !has_kea_dhcp6(&root) {
            return Err(MigrationError::KeaV6NotConfigured.into());
        }
        return Err(MigrationError::NoKeaSubnetsV6.into());
    }

    // Check fail_if_existing flag
    if options.fail_if_existing
        && (!existing_ips.is_empty()
            || !existing_ips_v6.is_empty()
            || !existing_duids_v6.is_empty())
    {
        return Err(anyhow!(
            "Existing reservations found ({} IPs) and --fail-if-existing is set. Aborting.",
            existing_ips.len() + existing_ips_v6.len()
        ));
    }

    let mut to_create = 0;
    let mut skipped = 0;

    // Track reserved IPs including ones we're adding (to catch ISC duplicates)
    let mut reserved_ips = existing_ips;

    if options.verbose {
        println!("\nProcessing {} ISC static mappings:", isc_mappings.len());
        if !isc_mappings_v6.is_empty() {
            println!(
                "Processing {} ISC DHCPv6 static mappings:",
                isc_mappings_v6.len()
            );
        }
    }

    let mut to_create_v6 = 0;
    let mut skipped_v6 = 0;
    let mut reserved_ips_v6 = existing_ips_v6;
    let mut reserved_duids_v6 = existing_duids_v6;

    if !isc_mappings.is_empty() {
        // Get the reservations node (fails if Kea/dhcp4 missing, creates <reservations> if needed)
        let reservations_node = get_reservations_node(&mut root)?;

        for mapping in &isc_mappings {
            if reserved_ips.contains(&mapping.ipaddr) {
                skipped += 1;
                if options.verbose {
                    println!(
                        "  SKIP: {} ({}) - IP already reserved",
                        mapping.ipaddr, mapping.mac
                    );
                }
                continue;
            }

            // Find matching subnet
            let subnet_uuid = find_subnet_for_ip(&mapping.ipaddr, &kea_subnets)?;

            if options.verbose {
                let hostname = mapping
                    .hostname
                    .as_ref()
                    .or(mapping.cid.as_ref())
                    .map(|s| s.as_str())
                    .unwrap_or("<no hostname>");
                println!(
                    "  ADD: {} ({}) -> subnet {} [{}]",
                    mapping.ipaddr,
                    mapping.mac,
                    short_uuid(&subnet_uuid),
                    hostname
                );
            }

            // Create and append reservation
            let reservation = create_reservation_element(mapping, &subnet_uuid);
            reservations_node
                .children
                .push(XMLNode::Element(reservation));
            reserved_ips.insert(mapping.ipaddr.clone());
            to_create += 1;
        }
    }

    if !isc_mappings_v6.is_empty() {
        let reservations_node_v6 = get_reservations_node_v6(&mut root)?;
        for mapping in &isc_mappings_v6 {
            if reserved_ips_v6.contains(&mapping.ipaddr)
                || reserved_duids_v6.contains(&mapping.duid)
            {
                skipped_v6 += 1;
                if options.verbose {
                    println!(
                        "  SKIP6: {} ({}) - IP or DUID already reserved",
                        mapping.ipaddr, mapping.duid
                    );
                }
                continue;
            }

            let subnet_uuid = find_subnet_for_ip_v6(&mapping.ipaddr, &kea_subnets_v6)?;

            if options.verbose {
                let hostname = mapping.hostname.as_deref().unwrap_or("<no hostname>");
                println!(
                    "  ADD6: {} ({}) -> subnet {} [{}]",
                    mapping.ipaddr,
                    mapping.duid,
                    short_uuid(&subnet_uuid),
                    hostname
                );
            }

            let reservation = create_reservation_element_v6(mapping, &subnet_uuid);
            reservations_node_v6
                .children
                .push(XMLNode::Element(reservation));
            reserved_ips_v6.insert(mapping.ipaddr.clone());
            reserved_duids_v6.insert(mapping.duid.clone());
            to_create_v6 += 1;
        }
    }

    // Write the updated XML with human-readable indentation
    let emitter_config = EmitterConfig::new()
        .perform_indent(true)
        .indent_string("  ")
        .write_document_declaration(true);
    root.write_with_config(writer, emitter_config)
        .context("Failed to write XML")?;

    Ok(MigrationStats {
        isc_mappings_found: isc_mappings.len(),
        isc_mappings_v6_found: isc_mappings_v6.len(),
        kea_subnets_found: kea_subnets.len(),
        kea_subnets_v6_found: kea_subnets_v6.len(),
        reservations_to_create: to_create,
        reservations_v6_to_create: to_create_v6,
        reservations_skipped: skipped,
        reservations_v6_skipped: skipped_v6,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fail_if_existing_flag() {
        let xml_with_existing = r#"<?xml version="1.0"?>
<opnsense>
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
}
