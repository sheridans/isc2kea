use anyhow::{anyhow, Result};
use xmltree::{Element, XMLNode};

use crate::extract::{
    extract_interface_cidrs, extract_interface_cidrs_v6, extract_isc_options_v4,
    extract_isc_options_v6,
};
use crate::extract_dnsmasq::{
    extract_existing_dnsmasq_client_ids, extract_existing_dnsmasq_ips,
    extract_existing_dnsmasq_macs, extract_existing_dnsmasq_options,
    extract_existing_dnsmasq_ranges, has_dnsmasq,
};
use crate::migrate_dnsmasq::{
    create_dnsmasq_host_element, create_dnsmasq_host_element_v6, create_dnsmasq_option_element,
    create_dnsmasq_range_element_v4, create_dnsmasq_range_element_v6, get_dnsmasq_node,
};
use crate::subnet::prefix_to_netmask;
use crate::{IscStaticMap, IscStaticMapV6, MigrationError, MigrationOptions, MigrationStats};

use super::options::{
    dnsmasq_option_key_from_elem, dnsmasq_option_specs_from_isc, DnsmasqOptionSpec,
};
use super::services::{disable_isc_dhcp_from_config, enable_dnsmasq};
use super::subnets::{
    cidr_prefix_v4, cidr_prefix_v6, desired_subnets_v4, desired_subnets_v6, DesiredSubnetV4,
    DesiredSubnetV6,
};
use super::utils::{validate_mapping_ifaces_v4, validate_mapping_ifaces_v6};

fn range_key(iface: &str, start: &str, end: &str, prefix_len: &str, mask: &str) -> String {
    format!("{}|{}|{}|{}|{}", iface, start, end, prefix_len, mask)
}

fn option_key_for_spec(spec: &DnsmasqOptionSpec) -> String {
    crate::extract_dnsmasq::dnsmasq_option_key(
        "set",
        &spec.option,
        &spec.option6,
        &spec.iface,
        "",
        "",
    )
}

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
        target_subnets_found: 0,
        target_subnets_v6_found: 0,
        reservations_to_create: to_create,
        reservations_v6_to_create: to_create_v6,
        reservations_skipped: skipped,
        reservations_v6_skipped: skipped_v6,
        ..Default::default()
    })
}

/// Convert an input configuration into dnsmasq hosts/ranges/options.
pub(crate) fn convert_dnsmasq(
    root: &mut Element,
    isc_mappings: &[IscStaticMap],
    isc_mappings_v6: &[IscStaticMapV6],
    options: &MigrationOptions,
) -> Result<MigrationStats> {
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
    let existing_options = if options.create_options {
        extract_existing_dnsmasq_options(root)?
    } else {
        std::collections::HashSet::new()
    };

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

    if !isc_mappings.is_empty()
        || !isc_mappings_v6.is_empty()
        || (options.create_subnets && (!desired_v4.is_empty() || !desired_v6.is_empty()))
        || (options.create_options && !desired_options.is_empty())
    {
        let dnsmasq_node = get_dnsmasq_node(root)?;

        if options.create_subnets {
            for subnet in &desired_v4 {
                let prefix = cidr_prefix_v4(&subnet.cidr)?;
                let mask = prefix_to_netmask(prefix)?;
                for range in &subnet.ranges {
                    let key = range_key(&subnet.iface, &range.from, &range.to, "", &mask);
                    if existing_ranges.contains(&key) {
                        if options.force_subnets {
                            dnsmasq_node.children.retain(|child| {
                                let Some(elem) = child.as_element() else {
                                    return true;
                                };
                                if !elem.name.eq_ignore_ascii_case("dhcp_ranges") {
                                    return true;
                                }
                                let iface = crate::xml_helpers::get_child_ci(elem, "interface")
                                    .and_then(|e| e.get_text())
                                    .map(|s| s.to_string())
                                    .unwrap_or_default();
                                let start = crate::xml_helpers::get_child_ci(elem, "start_addr")
                                    .and_then(|e| e.get_text())
                                    .map(|s| s.to_string())
                                    .unwrap_or_default();
                                let end = crate::xml_helpers::get_child_ci(elem, "end_addr")
                                    .and_then(|e| e.get_text())
                                    .map(|s| s.to_string())
                                    .unwrap_or_default();
                                let prefix_len =
                                    crate::xml_helpers::get_child_ci(elem, "prefix_len")
                                        .and_then(|e| e.get_text())
                                        .map(|s| s.to_string())
                                        .unwrap_or_default();
                                let subnet_mask =
                                    crate::xml_helpers::get_child_ci(elem, "subnet_mask")
                                        .and_then(|e| e.get_text())
                                        .map(|s| s.to_string())
                                        .unwrap_or_default();
                                let existing_key =
                                    range_key(&iface, &start, &end, &prefix_len, &subnet_mask);
                                existing_key != key
                            });
                        } else {
                            eprintln!(
                                "Warning: dnsmasq range {}-{} already exists (iface {}). Skipping.",
                                range.from, range.to, subnet.iface
                            );
                            continue;
                        }
                    }

                    let elem = create_dnsmasq_range_element_v4(
                        &subnet.iface,
                        &range.from,
                        &range.to,
                        &mask,
                    );
                    dnsmasq_node.children.push(XMLNode::Element(elem));
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
                        if options.force_subnets {
                            dnsmasq_node.children.retain(|child| {
                                let Some(elem) = child.as_element() else {
                                    return true;
                                };
                                if !elem.name.eq_ignore_ascii_case("dhcp_ranges") {
                                    return true;
                                }
                                let iface = crate::xml_helpers::get_child_ci(elem, "interface")
                                    .and_then(|e| e.get_text())
                                    .map(|s| s.to_string())
                                    .unwrap_or_default();
                                let start = crate::xml_helpers::get_child_ci(elem, "start_addr")
                                    .and_then(|e| e.get_text())
                                    .map(|s| s.to_string())
                                    .unwrap_or_default();
                                let end = crate::xml_helpers::get_child_ci(elem, "end_addr")
                                    .and_then(|e| e.get_text())
                                    .map(|s| s.to_string())
                                    .unwrap_or_default();
                                let prefix_len =
                                    crate::xml_helpers::get_child_ci(elem, "prefix_len")
                                        .and_then(|e| e.get_text())
                                        .map(|s| s.to_string())
                                        .unwrap_or_default();
                                let subnet_mask =
                                    crate::xml_helpers::get_child_ci(elem, "subnet_mask")
                                        .and_then(|e| e.get_text())
                                        .map(|s| s.to_string())
                                        .unwrap_or_default();
                                let existing_key =
                                    range_key(&iface, &start, &end, &prefix_len, &subnet_mask);
                                existing_key != key
                            });
                        } else {
                            eprintln!(
                                "Warning: dnsmasq range {}-{} already exists (iface {}). Skipping.",
                                range.from, range.to, subnet.iface
                            );
                            continue;
                        }
                    }

                    let elem = create_dnsmasq_range_element_v6(
                        &subnet.iface,
                        &range.from,
                        &range.to,
                        &prefix.to_string(),
                    );
                    dnsmasq_node.children.push(XMLNode::Element(elem));
                }
            }
        }

        if options.create_options {
            for spec in &desired_options {
                let key = option_key_for_spec(spec);
                if existing_options.contains(&key) {
                    if options.force_options {
                        dnsmasq_node.children.retain(|child| {
                            let Some(elem) = child.as_element() else {
                                return true;
                            };
                            let Some(existing_key) = dnsmasq_option_key_from_elem(elem) else {
                                return true;
                            };
                            existing_key != key
                        });
                    } else {
                        eprintln!(
                            "Warning: dnsmasq option {} already exists (iface {}). Skipping.",
                            if spec.option.is_empty() {
                                format!("v6:{}", spec.option6)
                            } else {
                                spec.option.clone()
                            },
                            spec.iface
                        );
                        continue;
                    }
                }

                let elem = create_dnsmasq_option_element(
                    &spec.iface,
                    &spec.option,
                    &spec.option6,
                    &spec.value,
                );
                dnsmasq_node.children.push(XMLNode::Element(elem));
            }
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
                continue;
            }

            if options.verbose {
                let hostname = mapping
                    .hostname
                    .as_ref()
                    .or(mapping.cid.as_ref())
                    .map(|s| s.as_str())
                    .unwrap_or("<no hostname>");
                println!("  ADD: {} ({}) [{}]", mapping.ipaddr, mapping.mac, hostname);
            }

            let host_elem = create_dnsmasq_host_element(mapping);
            dnsmasq_node.children.push(XMLNode::Element(host_elem));
            reserved_ips.insert(mapping.ipaddr.clone());
            reserved_macs.insert(mapping.mac.clone());
            to_create += 1;
        }

        for mapping in isc_mappings_v6 {
            if reserved_ips.contains(&mapping.ipaddr) || reserved_client_ids.contains(&mapping.duid)
            {
                skipped_v6 += 1;
                if options.verbose {
                    println!(
                        "  SKIP6: {} ({}) - IP or DUID already exists in dnsmasq",
                        mapping.ipaddr, mapping.duid
                    );
                }
                continue;
            }

            if options.verbose {
                let hostname = mapping.hostname.as_deref().unwrap_or("<no hostname>");
                println!(
                    "  ADD6: {} ({}) [{}]",
                    mapping.ipaddr, mapping.duid, hostname
                );
            }

            let host_elem = create_dnsmasq_host_element_v6(mapping);
            dnsmasq_node.children.push(XMLNode::Element(host_elem));
            reserved_ips.insert(mapping.ipaddr.clone());
            reserved_client_ids.insert(mapping.duid.clone());
            to_create_v6 += 1;
        }
    }

    let mut interfaces_configured = Vec::new();
    if options.create_subnets {
        interfaces_configured = apply_dnsmasq_interfaces(root, &desired_v4, &desired_v6)?;
    }

    let mut isc_disabled_v4 = Vec::new();
    let mut isc_disabled_v6 = Vec::new();
    let mut backend_enabled_v4 = false;
    let mut backend_enabled_v6 = false;
    if options.enable_backend {
        let (disabled_v4, disabled_v6) = disable_isc_dhcp_from_config(root)?;
        isc_disabled_v4 = disabled_v4;
        isc_disabled_v6 = disabled_v6;
        let has_ranges = !desired_v4.is_empty()
            || !desired_v6.is_empty()
            || !existing_ranges.is_empty();
        if has_ranges {
            backend_enabled_v4 = enable_dnsmasq(root)?;
            backend_enabled_v6 = backend_enabled_v4;
        }

        if has_ranges && !backend_enabled_v4 {
            return Err(anyhow!(
                "Failed to enable dnsmasq. Check that <dnsmasq> is present."
            ));
        }
    }

    Ok(MigrationStats {
        isc_mappings_found: isc_mappings.len(),
        isc_mappings_v6_found: isc_mappings_v6.len(),
        target_subnets_found: 0,
        target_subnets_v6_found: 0,
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

/// Populate `<dnsmasq><interface>` with interfaces from desired subnets.
///
/// This sets the global listening interfaces for dnsmasq DHCP service.
/// Without this, firewall rules won't be created for DHCP on those interfaces.
pub(crate) fn apply_dnsmasq_interfaces(
    root: &mut Element,
    desired_v4: &[DesiredSubnetV4],
    desired_v6: &[DesiredSubnetV6],
) -> Result<Vec<String>> {
    // Collect unique interfaces from both v4 and v6 subnets
    let mut ifaces: std::collections::HashSet<String> = std::collections::HashSet::new();
    for subnet in desired_v4 {
        ifaces.insert(subnet.iface.clone());
    }
    for subnet in desired_v6 {
        ifaces.insert(subnet.iface.clone());
    }

    if ifaces.is_empty() {
        return Ok(Vec::new());
    }

    let dnsmasq_node = get_dnsmasq_node(root)?;

    // Get existing interfaces and merge
    let existing = crate::xml_helpers::get_child_ci(dnsmasq_node, "interface")
        .and_then(|e| e.get_text())
        .map(|s| s.to_string())
        .unwrap_or_default();

    for iface in existing.split(',').filter(|s| !s.is_empty()) {
        ifaces.insert(iface.to_string());
    }

    // Remove existing interface element if present
    dnsmasq_node
        .children
        .retain(|c| c.as_element().is_none_or(|e| e.name != "interface"));

    // Add merged interfaces
    let mut ifaces_elem = Element::new("interface");
    let mut sorted_ifaces: Vec<_> = ifaces.into_iter().collect();
    sorted_ifaces.sort();
    ifaces_elem
        .children
        .push(XMLNode::Text(sorted_ifaces.join(",")));
    dnsmasq_node.children.push(XMLNode::Element(ifaces_elem));

    Ok(sorted_ifaces)
}
