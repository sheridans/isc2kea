use anyhow::{anyhow, Result};
use xmltree::{Element, XMLNode};

use crate::extract::{
    extract_existing_reservation_duids_v6, extract_existing_reservation_ips,
    extract_existing_reservation_ips_v6, extract_interface_cidrs, extract_interface_cidrs_v6,
    extract_isc_options_v4, extract_isc_options_v6, extract_kea_subnets, extract_kea_subnets_v6,
    has_kea_dhcp4, has_kea_dhcp6,
};
use crate::migrate_v4::{create_reservation_element, get_reservations_node};
use crate::migrate_v6::{create_reservation_element_v6, get_reservations_node_v6};
use crate::subnet::{find_subnet_for_ip, find_subnet_for_ip_v6};
use crate::{IscStaticMap, IscStaticMapV6, MigrationError, MigrationOptions, MigrationStats};

use super::options::apply_kea_options;
use super::services::{disable_isc_dhcp_from_config, enable_kea};
use super::subnets::{
    apply_kea_interfaces, apply_kea_subnets, desired_subnets_v4, desired_subnets_v6,
};
use super::utils::{short_uuid, validate_mapping_ifaces_v4, validate_mapping_ifaces_v6};

/// Scan an input configuration for Kea migration stats.
pub(crate) fn scan_kea(
    root: &Element,
    isc_mappings: &[IscStaticMap],
    isc_mappings_v6: &[IscStaticMapV6],
    options: &MigrationOptions,
) -> Result<MigrationStats> {
    let kea_subnets = extract_kea_subnets(root)?;
    let existing_ips = extract_existing_reservation_ips(root)?;
    let kea_subnets_v6 = extract_kea_subnets_v6(root)?;
    let existing_ips_v6 = extract_existing_reservation_ips_v6(root)?;
    let existing_duids_v6 = extract_existing_reservation_duids_v6(root)?;
    let iface_cidrs_v4 = extract_interface_cidrs(root)?;
    let iface_cidrs_v6 = extract_interface_cidrs_v6(root)?;
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

    // Early check: differentiate between "Kea not configured" vs "no subnets"
    if !isc_mappings.is_empty() && kea_subnets.is_empty() && !options.create_subnets {
        if !has_kea_dhcp4(root) {
            return Err(MigrationError::BackendNotConfigured {
                backend: "Kea".into(),
            }
            .into());
        }
        return Err(MigrationError::NoBackendSubnets {
            backend: "Kea".into(),
        }
        .into());
    }

    if !isc_mappings_v6.is_empty() && kea_subnets_v6.is_empty() && !options.create_subnets {
        if !has_kea_dhcp6(root) {
            return Err(MigrationError::BackendV6NotConfigured {
                backend: "Kea".into(),
            }
            .into());
        }
        return Err(MigrationError::NoBackendSubnetsV6 {
            backend: "Kea".into(),
        }
        .into());
    }

    if options.create_subnets && !isc_mappings.is_empty() && kea_subnets.is_empty() {
        if !has_kea_dhcp4(root) {
            return Err(MigrationError::BackendNotConfigured {
                backend: "Kea".into(),
            }
            .into());
        }
        if desired_v4.is_empty() {
            return Err(anyhow!(
                "No DHCPv4 ranges found to create Kea subnets. Configure ranges or subnets first."
            ));
        }
    }

    if options.create_subnets && !isc_mappings_v6.is_empty() && kea_subnets_v6.is_empty() {
        if !has_kea_dhcp6(root) {
            return Err(MigrationError::BackendV6NotConfigured {
                backend: "Kea".into(),
            }
            .into());
        }
        if desired_v6.is_empty() {
            return Err(anyhow!(
                "No DHCPv6 ranges found to create Kea subnets. Configure ranges or subnets first."
            ));
        }
    }

    if options.create_subnets && options.verbose {
        let existing_v4: std::collections::HashSet<_> =
            kea_subnets.iter().map(|s| s.cidr.clone()).collect();
        for subnet in &desired_v4 {
            if existing_v4.contains(&subnet.cidr) {
                eprintln!(
                    "Warning: Kea subnet {} already exists (iface {}). Skipping.",
                    subnet.cidr, subnet.iface
                );
            } else {
                println!("  ADD-SUBNET: {} (iface {})", subnet.cidr, subnet.iface);
            }
        }

        let existing_v6: std::collections::HashSet<_> =
            kea_subnets_v6.iter().map(|s| s.cidr.clone()).collect();
        for subnet in &desired_v6 {
            if existing_v6.contains(&subnet.cidr) {
                eprintln!(
                    "Warning: Kea subnet {} already exists (iface {}). Skipping.",
                    subnet.cidr, subnet.iface
                );
            } else {
                println!("  ADD-SUBNET6: {} (iface {})", subnet.cidr, subnet.iface);
            }
        }
    }

    validate_mapping_ifaces_v4(isc_mappings, &iface_cidrs_v4)?;
    validate_mapping_ifaces_v6(isc_mappings_v6, &iface_cidrs_v6)?;

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

    let mut effective_subnets = kea_subnets.clone();
    if options.create_subnets {
        for subnet in &desired_v4 {
            if !effective_subnets.iter().any(|s| s.cidr == subnet.cidr) {
                effective_subnets.push(crate::Subnet {
                    uuid: format!("new-{}", uuid::Uuid::new_v4()),
                    cidr: subnet.cidr.clone(),
                    iface: Some(subnet.iface.clone()),
                });
            }
        }
    }

    let mut effective_subnets_v6 = kea_subnets_v6.clone();
    if options.create_subnets {
        for subnet in &desired_v6 {
            if !effective_subnets_v6.iter().any(|s| s.cidr == subnet.cidr) {
                effective_subnets_v6.push(crate::SubnetV6 {
                    uuid: format!("new-{}", uuid::Uuid::new_v4()),
                    cidr: subnet.cidr.clone(),
                    iface: Some(subnet.iface.clone()),
                });
            }
        }
    }

    if options.verbose {
        println!("\nProcessing {} ISC static mappings:", isc_mappings.len());
        if !isc_mappings_v6.is_empty() {
            println!(
                "Processing {} ISC DHCPv6 static mappings:",
                isc_mappings_v6.len()
            );
        }
    }

    for mapping in isc_mappings {
        if reserved_ips.contains(&mapping.ipaddr) {
            skipped += 1;
            if options.verbose {
                println!(
                    "  SKIP: {} ({}) - IP already reserved",
                    mapping.ipaddr, mapping.mac
                );
            }
        } else {
            let subnet_uuid = find_subnet_for_ip(&mapping.ipaddr, &effective_subnets)?;
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

    for mapping in isc_mappings_v6 {
        if reserved_ips_v6.contains(&mapping.ipaddr) || reserved_duids_v6.contains(&mapping.duid) {
            skipped_v6 += 1;
            if options.verbose {
                println!(
                    "  SKIP6: {} ({}) - IP or DUID already reserved",
                    mapping.ipaddr, mapping.duid
                );
            }
        } else {
            let subnet_uuid = find_subnet_for_ip_v6(&mapping.ipaddr, &effective_subnets_v6)?;
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
        target_subnets_found: kea_subnets.len(),
        target_subnets_v6_found: kea_subnets_v6.len(),
        reservations_to_create: to_create,
        reservations_v6_to_create: to_create_v6,
        reservations_skipped: skipped,
        reservations_v6_skipped: skipped_v6,
        ..Default::default()
    })
}

/// Convert an input configuration into Kea reservations.
pub(crate) fn convert_kea(
    root: &mut Element,
    isc_mappings: &[IscStaticMap],
    isc_mappings_v6: &[IscStaticMapV6],
    options: &MigrationOptions,
) -> Result<MigrationStats> {
    let mut kea_subnets = extract_kea_subnets(root)?;
    let existing_ips = extract_existing_reservation_ips(root)?;
    let mut kea_subnets_v6 = extract_kea_subnets_v6(root)?;
    let existing_ips_v6 = extract_existing_reservation_ips_v6(root)?;
    let existing_duids_v6 = extract_existing_reservation_duids_v6(root)?;
    let iface_cidrs_v4 = extract_interface_cidrs(root)?;
    let iface_cidrs_v6 = extract_interface_cidrs_v6(root)?;
    let want_desired = options.create_subnets || options.enable_backend;
    let desired_v4 = if want_desired {
        desired_subnets_v4(root)?
    } else {
        Vec::new()
    };
    let desired_v6 = if want_desired {
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
    let mut interfaces_configured = Vec::new();
    if options.create_subnets {
        apply_kea_subnets(
            root,
            &mut kea_subnets,
            &mut kea_subnets_v6,
            &desired_v4,
            &desired_v6,
            options,
        )?;
        interfaces_configured = apply_kea_interfaces(root, &desired_v4, &desired_v6)?;
    }

    if options.create_options {
        apply_kea_options(root, &options_v4, &options_v6, options.force_options)?;
    }

    validate_mapping_ifaces_v4(isc_mappings, &iface_cidrs_v4)?;
    validate_mapping_ifaces_v6(isc_mappings_v6, &iface_cidrs_v6)?;

    // Early check: differentiate between "Kea not configured" vs "no subnets"
    if !isc_mappings.is_empty() && kea_subnets.is_empty() && !options.create_subnets {
        if !has_kea_dhcp4(root) {
            return Err(MigrationError::BackendNotConfigured {
                backend: "Kea".into(),
            }
            .into());
        }
        return Err(MigrationError::NoBackendSubnets {
            backend: "Kea".into(),
        }
        .into());
    }

    if !isc_mappings_v6.is_empty() && kea_subnets_v6.is_empty() && !options.create_subnets {
        if !has_kea_dhcp6(root) {
            return Err(MigrationError::BackendV6NotConfigured {
                backend: "Kea".into(),
            }
            .into());
        }
        return Err(MigrationError::NoBackendSubnetsV6 {
            backend: "Kea".into(),
        }
        .into());
    }

    if options.create_subnets && !isc_mappings.is_empty() && kea_subnets.is_empty() {
        if !has_kea_dhcp4(root) {
            return Err(MigrationError::BackendNotConfigured {
                backend: "Kea".into(),
            }
            .into());
        }
        if desired_v4.is_empty() {
            return Err(anyhow!(
                "No DHCPv4 ranges found to create Kea subnets. Configure ranges or subnets first."
            ));
        }
    }

    if options.create_subnets && !isc_mappings_v6.is_empty() && kea_subnets_v6.is_empty() {
        if !has_kea_dhcp6(root) {
            return Err(MigrationError::BackendV6NotConfigured {
                backend: "Kea".into(),
            }
            .into());
        }
        if desired_v6.is_empty() {
            return Err(anyhow!(
                "No DHCPv6 ranges found to create Kea subnets. Configure ranges or subnets first."
            ));
        }
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
        let reservations_node = get_reservations_node(root)?;

        for mapping in isc_mappings {
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

            let reservation = create_reservation_element(mapping, &subnet_uuid);
            reservations_node
                .children
                .push(XMLNode::Element(reservation));
            reserved_ips.insert(mapping.ipaddr.clone());
            to_create += 1;
        }
    }

    if !isc_mappings_v6.is_empty() {
        let reservations_node_v6 = get_reservations_node_v6(root)?;
        for mapping in isc_mappings_v6 {
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

    let mut isc_disabled_v4 = Vec::new();
    let mut isc_disabled_v6 = Vec::new();
    let mut backend_enabled_v4 = false;
    let mut backend_enabled_v6 = false;
    if options.enable_backend {
        let (disabled_v4, disabled_v6) = disable_isc_dhcp_from_config(root)?;
        isc_disabled_v4 = disabled_v4;
        isc_disabled_v6 = disabled_v6;
        let (enabled_v4, enabled_v6) =
            enable_kea(root, !kea_subnets.is_empty(), !kea_subnets_v6.is_empty())?;
        backend_enabled_v4 = enabled_v4;
        backend_enabled_v6 = enabled_v6;

        if !kea_subnets.is_empty() && !backend_enabled_v4 {
            return Err(anyhow!(
                "Failed to enable Kea DHCPv4. Check for missing <general><enabled>."
            ));
        }
        if !kea_subnets_v6.is_empty() && !backend_enabled_v6 {
            return Err(anyhow!(
                "Failed to enable Kea DHCPv6. Check for missing <general><enabled>."
            ));
        }
    }

    Ok(MigrationStats {
        isc_mappings_found: isc_mappings.len(),
        isc_mappings_v6_found: isc_mappings_v6.len(),
        target_subnets_found: kea_subnets.len(),
        target_subnets_v6_found: kea_subnets_v6.len(),
        reservations_to_create: to_create,
        reservations_v6_to_create: to_create_v6,
        reservations_skipped: skipped,
        reservations_v6_skipped: skipped_v6,
        interfaces_configured,
        isc_disabled_v4,
        isc_disabled_v6,
        backend_enabled_v4,
        backend_enabled_v6,
    })
}
