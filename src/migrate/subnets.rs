use anyhow::{anyhow, Result};
use std::str::FromStr;
use xmltree::{Element, XMLNode};

use crate::extract::{
    extract_interface_cidrs, extract_interface_cidrs_v6, extract_isc_ranges, extract_isc_ranges_v6,
    extract_kea_subnets, extract_kea_subnets_v6,
};
use crate::subnet::{ip_in_subnet, ip_in_subnet_v6};
use crate::{IscRangeV4, IscRangeV6, MigrationError, MigrationOptions};

#[derive(Debug, Clone)]
pub(crate) struct DesiredSubnetV4 {
    pub(crate) iface: String,
    pub(crate) cidr: String,
    pub(crate) ranges: Vec<IscRangeV4>,
}

#[derive(Debug, Clone)]
pub(crate) struct DesiredSubnetV6 {
    pub(crate) iface: String,
    pub(crate) cidr: String,
    pub(crate) ranges: Vec<IscRangeV6>,
}

pub(crate) fn desired_subnets_v4(root: &Element) -> Result<Vec<DesiredSubnetV4>> {
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

pub(crate) fn desired_subnets_v6(root: &Element) -> Result<Vec<DesiredSubnetV6>> {
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
    let pool_str = ranges
        .iter()
        .map(|r| format!("{}-{}", r.from, r.to))
        .collect::<Vec<_>>()
        .join(",");
    pools.children.push(XMLNode::Text(pool_str));
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
    let pool_str = ranges
        .iter()
        .map(|r| format!("{}-{}", r.from, r.to))
        .collect::<Vec<_>>()
        .join(",");
    pools.children.push(XMLNode::Text(pool_str));
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

pub(crate) fn apply_kea_subnets(
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

pub(crate) fn cidr_prefix_v4(cidr: &str) -> Result<u8> {
    let net = ipnet::Ipv4Net::from_str(cidr)
        .map_err(|_| MigrationError::InvalidCidr(cidr.to_string()))?;
    Ok(net.prefix_len())
}

pub(crate) fn cidr_prefix_v6(cidr: &str) -> Result<u8> {
    let net = ipnet::Ipv6Net::from_str(cidr)
        .map_err(|_| MigrationError::InvalidCidr(cidr.to_string()))?;
    Ok(net.prefix_len())
}
