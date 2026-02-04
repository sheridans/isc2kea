use anyhow::Result;
use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use xmltree::Element;

use crate::xml_helpers::get_child_ci;

/// Extract interface IPv4 CIDRs from the XML tree (interface name -> CIDR)
pub fn extract_interface_cidrs(root: &Element) -> Result<HashMap<String, String>> {
    let mut cidrs = HashMap::new();

    if let Some(interfaces) = get_child_ci(root, "interfaces") {
        for iface_node in interfaces.children.iter() {
            if let Some(iface_elem) = iface_node.as_element() {
                let iface_name = iface_elem.name.clone();
                let ipaddr = get_child_ci(iface_elem, "ipaddr")
                    .and_then(|e| e.get_text())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let subnet = get_child_ci(iface_elem, "subnet")
                    .and_then(|e| e.get_text())
                    .map(|s| s.to_string())
                    .unwrap_or_default();

                if ipaddr.is_empty() || subnet.is_empty() {
                    continue;
                }
                if ipaddr.eq_ignore_ascii_case("dhcp") {
                    continue;
                }

                let prefix: u8 = match subnet.parse() {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let ip = match Ipv4Addr::from_str(&ipaddr) {
                    Ok(ip) => ip,
                    Err(_) => continue,
                };

                let net = ipnet::Ipv4Net::new(ip, prefix)
                    .map_err(|_| crate::MigrationError::InvalidCidr(subnet.clone()))?;
                let cidr = format!("{}/{}", net.network(), net.prefix_len());
                cidrs.insert(iface_name, cidr);
            }
        }
    }

    Ok(cidrs)
}

/// Extract interface IPv6 CIDRs from the XML tree (interface name -> CIDR)
pub fn extract_interface_cidrs_v6(root: &Element) -> Result<HashMap<String, String>> {
    let mut cidrs = HashMap::new();

    if let Some(interfaces) = get_child_ci(root, "interfaces") {
        for iface_node in interfaces.children.iter() {
            if let Some(iface_elem) = iface_node.as_element() {
                let iface_name = iface_elem.name.clone();
                let ipaddr = get_child_ci(iface_elem, "ipaddrv6")
                    .and_then(|e| e.get_text())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let subnet = get_child_ci(iface_elem, "subnetv6")
                    .and_then(|e| e.get_text())
                    .map(|s| s.to_string())
                    .unwrap_or_default();

                if ipaddr.is_empty() || subnet.is_empty() {
                    continue;
                }
                let ipaddr_lower = ipaddr.to_ascii_lowercase();
                if ipaddr_lower == "dhcp6" || ipaddr_lower == "track6" {
                    continue;
                }

                let prefix: u8 = match subnet.parse() {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let ip = match Ipv6Addr::from_str(&ipaddr) {
                    Ok(ip) => ip,
                    Err(_) => continue,
                };

                let net = ipnet::Ipv6Net::new(ip, prefix)
                    .map_err(|_| crate::MigrationError::InvalidCidr(subnet.clone()))?;
                let cidr = format!("{}/{}", net.network(), net.prefix_len());
                cidrs.insert(iface_name, cidr);
            }
        }
    }

    Ok(cidrs)
}
