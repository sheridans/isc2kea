//! Functions to enable/disable DHCP services in OPNsense config.

use anyhow::Result;
use xmltree::{Element, XMLNode};

use super::subnets::{DesiredSubnetV4, DesiredSubnetV6};

/// Disable ISC DHCP on interfaces that have subnets being migrated.
///
/// Sets `<dhcpd><{iface}><enable></enable>` to empty for each interface.
/// Same for `<dhcpdv6>`.
pub(crate) fn disable_isc_dhcp(
    root: &mut Element,
    desired_v4: &[DesiredSubnetV4],
    desired_v6: &[DesiredSubnetV6],
) -> Result<()> {
    // Collect interfaces to disable
    let mut ifaces_v4: std::collections::HashSet<String> = std::collections::HashSet::new();
    for subnet in desired_v4 {
        ifaces_v4.insert(subnet.iface.clone());
    }

    let mut ifaces_v6: std::collections::HashSet<String> = std::collections::HashSet::new();
    for subnet in desired_v6 {
        ifaces_v6.insert(subnet.iface.clone());
    }

    // Disable DHCPv4 on migrated interfaces
    if !ifaces_v4.is_empty() {
        if let Some(dhcpd) = crate::xml_helpers::find_mut_descendant_ci(root, "dhcpd") {
            for iface in &ifaces_v4 {
                if let Some(iface_node) = crate::xml_helpers::get_mut_child_ci(dhcpd, iface) {
                    set_enable_element(iface_node, false);
                }
            }
        }
    }

    // Disable DHCPv6 on migrated interfaces
    if !ifaces_v6.is_empty() {
        if let Some(dhcpdv6) = crate::xml_helpers::find_mut_descendant_ci(root, "dhcpdv6") {
            for iface in &ifaces_v6 {
                if let Some(iface_node) = crate::xml_helpers::get_mut_child_ci(dhcpdv6, iface) {
                    set_enable_element(iface_node, false);
                }
            }
        }
    }

    Ok(())
}

/// Enable Kea DHCP services based on which protocols have subnets.
///
/// Only enables dhcp4 if v4 subnets exist, dhcp6 if v6 subnets exist.
pub(crate) fn enable_kea(
    root: &mut Element,
    has_v4_subnets: bool,
    has_v6_subnets: bool,
) -> Result<()> {
    let kea = match crate::xml_helpers::find_mut_descendant_ci(root, "Kea") {
        Some(kea) => kea,
        None => return Ok(()), // Kea not configured
    };

    if has_v4_subnets {
        if let Some(dhcp4) = crate::xml_helpers::get_mut_child_ci(kea, "dhcp4") {
            if let Some(general) = crate::xml_helpers::get_mut_child_ci(dhcp4, "general") {
                set_enable_element(general, true);
            }
        }
    }

    if has_v6_subnets {
        if let Some(dhcp6) = crate::xml_helpers::get_mut_child_ci(kea, "dhcp6") {
            if let Some(general) = crate::xml_helpers::get_mut_child_ci(dhcp6, "general") {
                set_enable_element(general, true);
            }
        }
    }

    Ok(())
}

/// Enable dnsmasq DHCP service.
pub(crate) fn enable_dnsmasq(root: &mut Element) -> Result<()> {
    let dnsmasq = match crate::xml_helpers::find_mut_descendant_ci(root, "dnsmasq") {
        Some(dnsmasq) => dnsmasq,
        None => return Ok(()), // dnsmasq not configured
    };

    set_enable_element(dnsmasq, true);
    Ok(())
}

/// Set the `<enable>` or `<enabled>` element within a node.
///
/// For ISC DHCP and dnsmasq, the element is `<enable>`.
/// For Kea general sections, the element is `<enabled>`.
fn set_enable_element(node: &mut Element, enabled: bool) {
    let value = if enabled { "1" } else { "" };

    // Try "enabled" first (Kea style), then "enable" (ISC/dnsmasq style)
    for tag in ["enabled", "enable"] {
        if let Some(elem) = crate::xml_helpers::get_mut_child_ci(node, tag) {
            elem.children.clear();
            elem.children.push(XMLNode::Text(value.to_string()));
            return;
        }
    }

    // If neither exists, create "enable" element (more common)
    let mut enable_elem = Element::new("enable");
    enable_elem.children.push(XMLNode::Text(value.to_string()));
    node.children.insert(0, XMLNode::Element(enable_elem));
}
