use anyhow::{anyhow, Result};
use xmltree::Element;

use crate::extract::{
    extract_interface_cidrs, extract_interface_cidrs_v6, extract_isc_options_v4,
    extract_isc_options_v6,
};
use crate::extract_dnsmasq::{
    extract_existing_dnsmasq_client_ids, extract_existing_dnsmasq_ips,
    extract_existing_dnsmasq_macs, extract_existing_dnsmasq_ranges, has_dnsmasq,
};
use crate::subnet::prefix_to_netmask;
use crate::{IscStaticMap, IscStaticMapV6, MigrationError, MigrationOptions, MigrationStats};

use super::range_key;
use crate::migrate::options::dnsmasq_option_specs_from_isc;
use crate::migrate::subnets::{
    cidr_prefix_v4, cidr_prefix_v6, desired_subnets_v4, desired_subnets_v6,
};
use crate::migrate::utils::{validate_mapping_ifaces_v4, validate_mapping_ifaces_v6};

/// Scan an input configuration for dnsmasq migration stats.
pub(crate) fn scan_dnsmasq(
    root: &Element,
    isc_mappings: &[IscStaticMap],
    isc_mappings_v6: &[IscStaticMapV6],
    options: &MigrationOptions,
) -> Result<MigrationStats> {
    let desired_v4 = if options.create_subnets {
        desired_subnets_v4(root)?
    } else {
        Vec::new()
    };
    let desired_v6 = if options.create_subnets {
        desired_subnets_v6(root)?
    } else {
        Vec::new()
    };
    let options_v4 = if options.create_options {
        extract_isc_options_v4(root)?
    } else {
        Vec::new()
    };
    let options_v6 = if options.create_options {
        extract_isc_options_v6(root)?
    } else {
        Vec::new()
    };
    let desired_options = if options.create_options {
        dnsmasq_option_specs_from_isc(&options_v4, &options_v6)
    } else {
        Vec::new()
    };
    let iface_cidrs_v4 = extract_interface_cidrs(root)?;
    let iface_cidrs_v6 = extract_interface_cidrs_v6(root)?;

    if (!isc_mappings.is_empty()
        || !isc_mappings_v6.is_empty()
        || !desired_v4.is_empty()
        || !desired_v6.is_empty()
        || !desired_options.is_empty())
        && !has_dnsmasq(root)
    {
        return Err(MigrationError::BackendNotConfigured {
            backend: "dnsmasq".into(),
        }
        .into());
    }

    let existing_ips = extract_existing_dnsmasq_ips(root)?;
    let existing_macs = extract_existing_dnsmasq_macs(root)?;
    let existing_client_ids = extract_existing_dnsmasq_client_ids(root)?;
    let existing_ranges = extract_existing_dnsmasq_ranges(root)?;

    if options.fail_if_existing
        && (!existing_ips.is_empty()
            || !existing_macs.is_empty()
            || !existing_client_ids.is_empty()
            || (options.create_subnets && !existing_ranges.is_empty()))
    {
        return Err(anyhow!(
            "Existing dnsmasq hosts found ({} entries) and --fail-if-existing is set. Aborting.",
            existing_ips.len()
        ));
    }

    let mut to_create = 0;
    let mut skipped = 0;
    let mut to_create_v6 = 0;
    let mut skipped_v6 = 0;
    let mut reserved_ips = existing_ips;
    let mut reserved_macs = existing_macs;
    let mut reserved_client_ids = existing_client_ids;

    validate_mapping_ifaces_v4(isc_mappings, &iface_cidrs_v4)?;
    validate_mapping_ifaces_v6(isc_mappings_v6, &iface_cidrs_v6)?;

    if options.verbose {
        println!(
            "\nProcessing {} ISC static mappings for dnsmasq:",
            isc_mappings.len()
        );
    }

    for mapping in isc_mappings {
        if reserved_ips.contains(&mapping.ipaddr) || reserved_macs.contains(&mapping.mac) {
            skipped += 1;
            if options.verbose {
                println!(
                    "  SKIP: {} ({}) - IP or MAC already exists in dnsmasq",
                    mapping.ipaddr, mapping.mac
                );
            }
        } else {
            reserved_ips.insert(mapping.ipaddr.clone());
            reserved_macs.insert(mapping.mac.clone());
            to_create += 1;
            if options.verbose {
                let hostname = mapping
                    .hostname
                    .as_ref()
                    .or(mapping.cid.as_ref())
                    .map(|s| s.as_str())
                    .unwrap_or("<no hostname>");
                println!("  ADD: {} ({}) [{}]", mapping.ipaddr, mapping.mac, hostname);
            }
        }
    }

    if options.create_subnets {
        for subnet in &desired_v4 {
            let prefix = cidr_prefix_v4(&subnet.cidr)?;
            let mask = prefix_to_netmask(prefix)?;
            for range in &subnet.ranges {
                let key = range_key(&subnet.iface, &range.from, &range.to, "", &mask);
                if existing_ranges.contains(&key) {
                    eprintln!(
                        "Warning: dnsmasq range {}-{} already exists (iface {}). Skipping.",
                        range.from, range.to, subnet.iface
                    );
                } else if options.verbose {
                    println!(
                        "  ADD-RANGE: {}-{} (iface {})",
                        range.from, range.to, subnet.iface
                    );
                }
            }
        }

        for subnet in &desired_v6 {
            let prefix = cidr_prefix_v6(&subnet.cidr)?;
            for range in &subnet.ranges {
                let key = range_key(
                    &subnet.iface,
                    &range.from,
                    &range.to,
                    &prefix.to_string(),
                    "",
                );
                if existing_ranges.contains(&key) {
                    eprintln!(
                        "Warning: dnsmasq range {}-{} already exists (iface {}). Skipping.",
                        range.from, range.to, subnet.iface
                    );
                } else if options.verbose {
                    println!(
                        "  ADD-RANGE6: {}-{} (iface {})",
                        range.from, range.to, subnet.iface
                    );
                }
            }
        }
    }

    if options.verbose {
        println!(
            "\nProcessing {} ISC DHCPv6 static mappings for dnsmasq:",
            isc_mappings_v6.len()
        );
    }

    for mapping in isc_mappings_v6 {
        if reserved_ips.contains(&mapping.ipaddr) || reserved_client_ids.contains(&mapping.duid) {
            skipped_v6 += 1;
            if options.verbose {
                println!(
                    "  SKIP6: {} ({}) - IP or DUID already exists in dnsmasq",
                    mapping.ipaddr, mapping.duid
                );
            }
        } else {
            reserved_ips.insert(mapping.ipaddr.clone());
            reserved_client_ids.insert(mapping.duid.clone());
            to_create_v6 += 1;
            if options.verbose {
                let hostname = mapping.hostname.as_deref().unwrap_or("<no hostname>");
                println!(
                    "  ADD6: {} ({}) [{}]",
                    mapping.ipaddr, mapping.duid, hostname
                );
            }
        }
    }

    Ok(MigrationStats {
        isc_mappings_found: isc_mappings.len(),
        isc_mappings_v6_found: isc_mappings_v6.len(),
        isc_ranges_found: 0,
        isc_ranges_v6_found: 0,
        target_subnets_found: 0,
        target_subnets_v6_found: 0,
        reservations_to_create: to_create,
        reservations_v6_to_create: to_create_v6,
        reservations_skipped: skipped,
        reservations_v6_skipped: skipped_v6,
        ..Default::default()
    })
}
