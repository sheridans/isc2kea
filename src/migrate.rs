use anyhow::{anyhow, Context, Result};
use std::io::{Read, Write};
use std::str::FromStr;
use xmltree::{Element, EmitterConfig, XMLNode};

use crate::backend::Backend;
use crate::extract::{
    extract_existing_reservation_duids_v6, extract_existing_reservation_ips,
    extract_existing_reservation_ips_v6, extract_interface_cidrs, extract_interface_cidrs_v6,
    extract_isc_mappings, extract_isc_mappings_v6, extract_isc_ranges, extract_isc_ranges_v6,
    extract_kea_subnets, extract_kea_subnets_v6, has_kea_dhcp4, has_kea_dhcp6,
};
use crate::extract_dnsmasq::{
    extract_existing_dnsmasq_client_ids, extract_existing_dnsmasq_ips,
    extract_existing_dnsmasq_macs, extract_existing_dnsmasq_ranges, has_dnsmasq,
};
use crate::migrate_dnsmasq::{
    create_dnsmasq_host_element, create_dnsmasq_host_element_v6, create_dnsmasq_range_element_v4,
    create_dnsmasq_range_element_v6, get_dnsmasq_node,
};
use crate::migrate_v4::{create_reservation_element, get_reservations_node};
use crate::migrate_v6::{create_reservation_element_v6, get_reservations_node_v6};
use crate::subnet::{
    find_subnet_for_ip, find_subnet_for_ip_v6, ip_in_subnet, ip_in_subnet_v6, prefix_to_netmask,
};
use crate::{
    IscRangeV4, IscRangeV6, IscStaticMap, IscStaticMapV6, MigrationError, MigrationOptions,
    MigrationStats,
};

fn short_uuid(uuid: &str) -> &str {
    uuid.get(..8).unwrap_or(uuid)
}

#[derive(Debug, Clone)]
struct DesiredSubnetV4 {
    iface: String,
    cidr: String,
    ranges: Vec<IscRangeV4>,
}

#[derive(Debug, Clone)]
struct DesiredSubnetV6 {
    iface: String,
    cidr: String,
    ranges: Vec<IscRangeV6>,
}

fn desired_subnets_v4(root: &Element) -> Result<Vec<DesiredSubnetV4>> {
    let ranges = extract_isc_ranges(root)?;
    if ranges.is_empty() {
        return Ok(Vec::new());
    }

    let iface_cidrs = extract_interface_cidrs(root)?;
    let mut by_iface: std::collections::HashMap<String, DesiredSubnetV4> =
        std::collections::HashMap::new();

    for range in ranges {
        let cidr = iface_cidrs.get(&range.iface).cloned().ok_or_else(|| {
            anyhow!(
                "No interface CIDR found for DHCPv4 interface '{}'",
                range.iface
            )
        })?;

        if !ip_in_subnet(&range.from, &cidr)? || !ip_in_subnet(&range.to, &cidr)? {
            return Err(anyhow!(
                "DHCPv4 range {}-{} is not contained within interface subnet {} ({})",
                range.from,
                range.to,
                range.iface,
                cidr
            ));
        }

        by_iface
            .entry(range.iface.clone())
            .and_modify(|entry| entry.ranges.push(range.clone()))
            .or_insert(DesiredSubnetV4 {
                iface: range.iface.clone(),
                cidr,
                ranges: vec![range],
            });
    }

    Ok(by_iface.into_values().collect())
}

fn desired_subnets_v6(root: &Element) -> Result<Vec<DesiredSubnetV6>> {
    let ranges = extract_isc_ranges_v6(root)?;
    if ranges.is_empty() {
        return Ok(Vec::new());
    }

    let iface_cidrs = extract_interface_cidrs_v6(root)?;
    let mut by_iface: std::collections::HashMap<String, DesiredSubnetV6> =
        std::collections::HashMap::new();

    for range in ranges {
        let cidr = iface_cidrs.get(&range.iface).cloned().ok_or_else(|| {
            anyhow!(
                "No interface CIDR found for DHCPv6 interface '{}'",
                range.iface
            )
        })?;

        if !ip_in_subnet_v6(&range.from, &cidr)? || !ip_in_subnet_v6(&range.to, &cidr)? {
            return Err(anyhow!(
                "DHCPv6 range {}-{} is not contained within interface subnet {} ({})",
                range.from,
                range.to,
                range.iface,
                cidr
            ));
        }

        by_iface
            .entry(range.iface.clone())
            .and_modify(|entry| entry.ranges.push(range.clone()))
            .or_insert(DesiredSubnetV6 {
                iface: range.iface.clone(),
                cidr,
                ranges: vec![range],
            });
    }

    Ok(by_iface.into_values().collect())
}

fn get_kea_subnets_node_mut(root: &mut Element, v6: bool) -> Result<&mut Element> {
    let kea = crate::xml_helpers::find_mut_descendant_ci(root, "Kea")
        .ok_or_else(|| anyhow!("Kea not configured in config.xml"))?;
    let dhcp_name = if v6 { "dhcp6" } else { "dhcp4" };
    let dhcp = crate::xml_helpers::find_mut_descendant_ci(kea, dhcp_name)
        .ok_or_else(|| anyhow!("Failed to access Kea {} node", dhcp_name))?;

    if crate::xml_helpers::get_mut_child_ci(dhcp, "subnets").is_none() {
        dhcp.children
            .push(XMLNode::Element(Element::new("subnets")));
    }

    crate::xml_helpers::get_mut_child_ci(dhcp, "subnets")
        .ok_or_else(|| anyhow!("Failed to access Kea subnets node"))
}

fn create_kea_subnet4_element(cidr: &str, ranges: &[IscRangeV4]) -> Element {
    let mut subnet4 = Element::new("subnet4");
    subnet4
        .attributes
        .insert("uuid".to_string(), uuid::Uuid::new_v4().to_string());

    let mut subnet_elem = Element::new("subnet");
    subnet_elem.children.push(XMLNode::Text(cidr.to_string()));
    subnet4.children.push(XMLNode::Element(subnet_elem));

    let mut pools = Element::new("pools");
    for range in ranges {
        let mut pool = Element::new("pool");
        pool.attributes
            .insert("uuid".to_string(), uuid::Uuid::new_v4().to_string());
        let mut pool_value = Element::new("pool");
        pool_value
            .children
            .push(XMLNode::Text(format!("{} - {}", range.from, range.to)));
        pool.children.push(XMLNode::Element(pool_value));
        pools.children.push(XMLNode::Element(pool));
    }
    subnet4.children.push(XMLNode::Element(pools));

    subnet4
}

fn create_kea_subnet6_element(cidr: &str, ranges: &[IscRangeV6]) -> Element {
    let mut subnet6 = Element::new("subnet6");
    subnet6
        .attributes
        .insert("uuid".to_string(), uuid::Uuid::new_v4().to_string());

    let mut subnet_elem = Element::new("subnet");
    subnet_elem.children.push(XMLNode::Text(cidr.to_string()));
    subnet6.children.push(XMLNode::Element(subnet_elem));

    let mut pools = Element::new("pools");
    for range in ranges {
        let mut pool = Element::new("pool");
        pool.attributes
            .insert("uuid".to_string(), uuid::Uuid::new_v4().to_string());
        let mut pool_value = Element::new("pool");
        pool_value
            .children
            .push(XMLNode::Text(format!("{} - {}", range.from, range.to)));
        pool.children.push(XMLNode::Element(pool_value));
        pools.children.push(XMLNode::Element(pool));
    }
    subnet6.children.push(XMLNode::Element(pools));

    subnet6
}

fn remove_kea_subnet_by_cidr(subnets_node: &mut Element, v6: bool, cidr: &str) -> bool {
    let subnet_tag = if v6 { "subnet6" } else { "subnet4" };
    let before = subnets_node.children.len();
    subnets_node.children.retain(|child| {
        let Some(elem) = child.as_element() else {
            return true;
        };
        if !elem.name.eq_ignore_ascii_case(subnet_tag) {
            return true;
        }
        let subnet_val = elem
            .children
            .iter()
            .filter_map(|c| c.as_element())
            .find(|e| e.name.eq_ignore_ascii_case("subnet"))
            .and_then(|e| e.get_text())
            .map(|s| s.to_string())
            .unwrap_or_default();
        subnet_val != cidr
    });
    before != subnets_node.children.len()
}

fn apply_kea_subnets(
    root: &mut Element,
    kea_subnets: &mut Vec<crate::Subnet>,
    kea_subnets_v6: &mut Vec<crate::SubnetV6>,
    desired_v4: &[DesiredSubnetV4],
    desired_v6: &[DesiredSubnetV6],
    options: &MigrationOptions,
) -> Result<()> {
    if !desired_v4.is_empty() {
        let existing: std::collections::HashSet<_> =
            kea_subnets.iter().map(|s| s.cidr.clone()).collect();
        let subnets_node = get_kea_subnets_node_mut(root, false)?;
        for subnet in desired_v4 {
            if existing.contains(&subnet.cidr) {
                if options.force_subnets {
                    remove_kea_subnet_by_cidr(subnets_node, false, &subnet.cidr);
                } else {
                    eprintln!(
                        "Warning: Kea subnet {} already exists (iface {}). Skipping.",
                        subnet.cidr, subnet.iface
                    );
                    continue;
                }
            }
            let elem = create_kea_subnet4_element(&subnet.cidr, &subnet.ranges);
            subnets_node.children.push(XMLNode::Element(elem));
        }
    }

    if !desired_v6.is_empty() {
        let existing: std::collections::HashSet<_> =
            kea_subnets_v6.iter().map(|s| s.cidr.clone()).collect();
        let subnets_node = get_kea_subnets_node_mut(root, true)?;
        for subnet in desired_v6 {
            if existing.contains(&subnet.cidr) {
                if options.force_subnets {
                    remove_kea_subnet_by_cidr(subnets_node, true, &subnet.cidr);
                } else {
                    eprintln!(
                        "Warning: Kea subnet {} already exists (iface {}). Skipping.",
                        subnet.cidr, subnet.iface
                    );
                    continue;
                }
            }
            let elem = create_kea_subnet6_element(&subnet.cidr, &subnet.ranges);
            subnets_node.children.push(XMLNode::Element(elem));
        }
    }

    *kea_subnets = extract_kea_subnets(root)?;
    *kea_subnets_v6 = extract_kea_subnets_v6(root)?;
    Ok(())
}

fn cidr_prefix_v4(cidr: &str) -> Result<u8> {
    let net = ipnet::Ipv4Net::from_str(cidr)
        .map_err(|_| MigrationError::InvalidCidr(cidr.to_string()))?;
    Ok(net.prefix_len())
}

fn cidr_prefix_v6(cidr: &str) -> Result<u8> {
    let net = ipnet::Ipv6Net::from_str(cidr)
        .map_err(|_| MigrationError::InvalidCidr(cidr.to_string()))?;
    Ok(net.prefix_len())
}

/// Scan the configuration and return basic counts without validation
pub fn scan_counts<R: Read>(reader: R, backend: &Backend) -> Result<MigrationStats> {
    let root = Element::parse(reader).context("Failed to parse XML")?;

    let isc_mappings = extract_isc_mappings(&root)?;
    let isc_mappings_v6 = extract_isc_mappings_v6(&root)?;

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
        target_subnets_found,
        target_subnets_v6_found,
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
    let isc_mappings_v6 = extract_isc_mappings_v6(&root)?;

    match options.backend {
        Backend::Kea => scan_kea(&root, &isc_mappings, &isc_mappings_v6, options),
        Backend::Dnsmasq => scan_dnsmasq(&root, &isc_mappings, &isc_mappings_v6, options),
    }
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
        Backend::Kea => convert_kea(&mut root, &isc_mappings, &isc_mappings_v6, options)?,
        Backend::Dnsmasq => convert_dnsmasq(&mut root, &isc_mappings, &isc_mappings_v6, options)?,
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

// ---------------------------------------------------------------------------
// Kea backend
// ---------------------------------------------------------------------------

fn scan_kea(
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
    })
}

fn convert_kea(
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
    if options.create_subnets {
        apply_kea_subnets(
            root,
            &mut kea_subnets,
            &mut kea_subnets_v6,
            &desired_v4,
            &desired_v6,
            options,
        )?;
    }

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

    Ok(MigrationStats {
        isc_mappings_found: isc_mappings.len(),
        isc_mappings_v6_found: isc_mappings_v6.len(),
        target_subnets_found: kea_subnets.len(),
        target_subnets_v6_found: kea_subnets_v6.len(),
        reservations_to_create: to_create,
        reservations_v6_to_create: to_create_v6,
        reservations_skipped: skipped,
        reservations_v6_skipped: skipped_v6,
    })
}

// ---------------------------------------------------------------------------
// dnsmasq backend
// ---------------------------------------------------------------------------

fn scan_dnsmasq(
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

    if (!isc_mappings.is_empty()
        || !isc_mappings_v6.is_empty()
        || !desired_v4.is_empty()
        || !desired_v6.is_empty())
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
        let range_key = |iface: &str, start: &str, end: &str, prefix_len: &str, mask: &str| {
            format!("{}|{}|{}|{}|{}", iface, start, end, prefix_len, mask)
        };

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
    })
}

fn convert_dnsmasq(
    root: &mut Element,
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

    if (!isc_mappings.is_empty()
        || !isc_mappings_v6.is_empty()
        || !desired_v4.is_empty()
        || !desired_v6.is_empty())
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

    if options.verbose {
        println!(
            "\nProcessing {} ISC static mappings for dnsmasq:",
            isc_mappings.len()
        );
    }

    if !isc_mappings.is_empty()
        || !isc_mappings_v6.is_empty()
        || (options.create_subnets && (!desired_v4.is_empty() || !desired_v6.is_empty()))
    {
        let dnsmasq_node = get_dnsmasq_node(root)?;

        if options.create_subnets {
            let range_key = |iface: &str, start: &str, end: &str, prefix_len: &str, mask: &str| {
                format!("{}|{}|{}|{}|{}", iface, start, end, prefix_len, mask)
            };

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

    Ok(MigrationStats {
        isc_mappings_found: isc_mappings.len(),
        isc_mappings_v6_found: isc_mappings_v6.len(),
        target_subnets_found: 0,
        target_subnets_v6_found: 0,
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
}
