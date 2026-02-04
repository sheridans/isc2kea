use anyhow::Result;
use std::collections::HashSet;
use xmltree::Element;

use crate::xml_helpers::{find_descendant_ci, get_child_ci};

/// Check if dnsmasq is configured in the XML tree
pub(crate) fn has_dnsmasq(root: &Element) -> bool {
    find_descendant_ci(root, "dnsmasq").is_some()
}

/// Extract existing dnsmasq host IP addresses for duplicate detection
pub(crate) fn extract_existing_dnsmasq_ips(root: &Element) -> Result<HashSet<String>> {
    let mut ips = HashSet::new();

    if let Some(dnsmasq) = find_descendant_ci(root, "dnsmasq") {
        for child in &dnsmasq.children {
            if let Some(host) = child.as_element() {
                if host.name.eq_ignore_ascii_case("hosts") {
                    if let Some(ip_elem) = get_child_ci(host, "ip") {
                        if let Some(ip) = ip_elem.get_text() {
                            let ip_str = ip.to_string();
                            if !ip_str.is_empty() {
                                ips.insert(ip_str);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(ips)
}

/// Extract existing dnsmasq host MAC addresses for duplicate detection
pub(crate) fn extract_existing_dnsmasq_macs(root: &Element) -> Result<HashSet<String>> {
    let mut macs = HashSet::new();

    if let Some(dnsmasq) = find_descendant_ci(root, "dnsmasq") {
        for child in &dnsmasq.children {
            if let Some(host) = child.as_element() {
                if host.name.eq_ignore_ascii_case("hosts") {
                    if let Some(mac_elem) = get_child_ci(host, "hwaddr") {
                        if let Some(mac) = mac_elem.get_text() {
                            let mac_str = mac.to_string();
                            if !mac_str.is_empty() {
                                macs.insert(mac_str);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(macs)
}

/// Extract existing dnsmasq client IDs (DUIDs) for duplicate detection
pub(crate) fn extract_existing_dnsmasq_client_ids(root: &Element) -> Result<HashSet<String>> {
    let mut client_ids = HashSet::new();

    if let Some(dnsmasq) = find_descendant_ci(root, "dnsmasq") {
        for child in &dnsmasq.children {
            if let Some(host) = child.as_element() {
                if host.name.eq_ignore_ascii_case("hosts") {
                    if let Some(cid_elem) = get_child_ci(host, "client_id") {
                        if let Some(cid) = cid_elem.get_text() {
                            let cid_str = cid.to_string();
                            if !cid_str.is_empty() {
                                client_ids.insert(cid_str);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(client_ids)
}
