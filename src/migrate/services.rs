//! Functions to enable/disable DHCP services in OPNsense config.

use anyhow::Result;
use xmltree::{Element, XMLNode};

use std::collections::HashSet;

/// Disable ISC DHCP on interfaces that are enabled in the current config.
///
/// Sets `<dhcpd><{iface}><enable></enable>` to empty for each interface.
/// Same for `<dhcpdv6>`.
///
/// Returns (disabled_v4_interfaces, disabled_v6_interfaces).
pub(crate) fn disable_isc_dhcp_from_config(root: &mut Element) -> Result<(Vec<String>, Vec<String>)> {
    let ifaces_v4 = isc_enabled_ifaces(root, "dhcpd");
    let ifaces_v6 = isc_enabled_ifaces(root, "dhcpdv6");
    disable_isc_dhcp_ifaces(root, &ifaces_v4, &ifaces_v6)
}

fn isc_enabled_ifaces(root: &Element, tag: &str) -> Vec<String> {
    let mut ifaces = HashSet::new();
    let Some(dhcp) = crate::xml_helpers::find_descendant_ci(root, tag) else {
        return Vec::new();
    };

    for iface_node in dhcp.children.iter().filter_map(|c| c.as_element()) {
        let iface_name = iface_node.name.clone();
        let enabled = match crate::xml_helpers::get_child_ci(iface_node, "enable")
            .and_then(|e| e.get_text())
        {
            Some(v) => {
                let val = v.trim();
                !(val.is_empty() || val == "0")
            }
            None => true,
        };
        if enabled {
            ifaces.insert(iface_name);
        }
    }

    let mut result: Vec<_> = ifaces.into_iter().collect();
    result.sort();
    result
}

fn disable_isc_dhcp_ifaces(
    root: &mut Element,
    ifaces_v4: &[String],
    ifaces_v6: &[String],
) -> Result<(Vec<String>, Vec<String>)> {
    let mut disabled_v4 = Vec::new();
    let mut disabled_v6 = Vec::new();

    if !ifaces_v4.is_empty() {
        if let Some(dhcpd) = crate::xml_helpers::find_mut_descendant_ci(root, "dhcpd") {
            for iface in ifaces_v4 {
                if let Some(iface_node) = crate::xml_helpers::get_mut_child_ci(dhcpd, iface) {
                    set_enable_element_generic(iface_node, false);
                    disabled_v4.push(iface.clone());
                }
            }
        }
    }

    if !ifaces_v6.is_empty() {
        if let Some(dhcpdv6) = crate::xml_helpers::find_mut_descendant_ci(root, "dhcpdv6") {
            for iface in ifaces_v6 {
                if let Some(iface_node) = crate::xml_helpers::get_mut_child_ci(dhcpdv6, iface) {
                    set_enable_element_generic(iface_node, false);
                    disabled_v6.push(iface.clone());
                }
            }
        }
    }

    disabled_v4.sort();
    disabled_v6.sort();
    Ok((disabled_v4, disabled_v6))
}

/// Enable Kea DHCP services based on which protocols have subnets.
///
/// Only enables dhcp4 if v4 subnets exist, dhcp6 if v6 subnets exist.
/// Returns (enabled_v4, enabled_v6).
pub(crate) fn enable_kea(
    root: &mut Element,
    has_v4_subnets: bool,
    has_v6_subnets: bool,
) -> Result<(bool, bool)> {
    let kea = match crate::xml_helpers::find_mut_descendant_ci(root, "Kea") {
        Some(kea) => kea,
        None => return Ok((false, false)), // Kea not configured
    };

    let mut enabled_v4 = false;
    let mut enabled_v6 = false;

    if has_v4_subnets {
        if let Some(dhcp4) = crate::xml_helpers::get_mut_child_ci(kea, "dhcp4") {
            if crate::xml_helpers::get_mut_child_ci(dhcp4, "general").is_none() {
                dhcp4
                    .children
                    .push(XMLNode::Element(Element::new("general")));
            }
            if let Some(general) = crate::xml_helpers::get_mut_child_ci(dhcp4, "general") {
                set_enable_element_kea(general, true);
                enabled_v4 = true;
            }
        }
    }

    if has_v6_subnets {
        if let Some(dhcp6) = crate::xml_helpers::get_mut_child_ci(kea, "dhcp6") {
            if crate::xml_helpers::get_mut_child_ci(dhcp6, "general").is_none() {
                dhcp6
                    .children
                    .push(XMLNode::Element(Element::new("general")));
            }
            if let Some(general) = crate::xml_helpers::get_mut_child_ci(dhcp6, "general") {
                set_enable_element_kea(general, true);
                enabled_v6 = true;
            }
        }
    }

    Ok((enabled_v4, enabled_v6))
}

/// Enable dnsmasq DHCP service.
/// Returns true if dnsmasq was enabled.
pub(crate) fn enable_dnsmasq(root: &mut Element) -> Result<bool> {
    let dnsmasq = match crate::xml_helpers::find_mut_descendant_ci(root, "dnsmasq") {
        Some(dnsmasq) => dnsmasq,
        None => return Ok(false), // dnsmasq not configured
    };

    set_enable_element_generic(dnsmasq, true);
    Ok(true)
}

/// Set the `<enable>` element within a node (ISC/dnsmasq).
fn set_enable_element_generic(node: &mut Element, enabled: bool) {
    let value = if enabled { "1" } else { "" };

    if let Some(elem) = crate::xml_helpers::get_mut_child_ci(node, "enable") {
        elem.children.clear();
        elem.children.push(XMLNode::Text(value.to_string()));
        return;
    }

    let mut enable_elem = Element::new("enable");
    enable_elem.children.push(XMLNode::Text(value.to_string()));
    node.children.insert(0, XMLNode::Element(enable_elem));
}

/// Set the `<enabled>` element within a Kea general node.
fn set_enable_element_kea(node: &mut Element, enabled: bool) {
    let value = if enabled { "1" } else { "" };

    if let Some(elem) = crate::xml_helpers::get_mut_child_ci(node, "enabled") {
        elem.children.clear();
        elem.children.push(XMLNode::Text(value.to_string()));
        return;
    }

    let mut enable_elem = Element::new("enabled");
    enable_elem.children.push(XMLNode::Text(value.to_string()));
    node.children.insert(0, XMLNode::Element(enable_elem));
}
