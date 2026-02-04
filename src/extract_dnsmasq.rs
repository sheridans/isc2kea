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

/// Extract existing dnsmasq DHCP ranges for duplicate detection
pub(crate) fn extract_existing_dnsmasq_ranges(root: &Element) -> Result<HashSet<String>> {
    let mut ranges = HashSet::new();

    if let Some(dnsmasq) = find_descendant_ci(root, "dnsmasq") {
        for child in &dnsmasq.children {
            if let Some(range) = child.as_element() {
                if range.name.eq_ignore_ascii_case("dhcp_ranges") {
                    let iface = get_child_ci(range, "interface")
                        .and_then(|e| e.get_text())
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let start = get_child_ci(range, "start_addr")
                        .and_then(|e| e.get_text())
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let end = get_child_ci(range, "end_addr")
                        .and_then(|e| e.get_text())
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let prefix_len = get_child_ci(range, "prefix_len")
                        .and_then(|e| e.get_text())
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let subnet_mask = get_child_ci(range, "subnet_mask")
                        .and_then(|e| e.get_text())
                        .map(|s| s.to_string())
                        .unwrap_or_default();

                    if iface.is_empty() || start.is_empty() || end.is_empty() {
                        continue;
                    }

                    let key = format!("{}|{}|{}|{}|{}", iface, start, end, prefix_len, subnet_mask);
                    ranges.insert(key);
                }
            }
        }
    }

    Ok(ranges)
}

pub(crate) fn dnsmasq_option_key(
    opt_type: &str,
    option: &str,
    option6: &str,
    iface: &str,
    tag: &str,
    set_tag: &str,
) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}",
        opt_type, option, option6, iface, tag, set_tag
    )
}

/// Extract existing dnsmasq DHCP options (type=set) for duplicate detection
pub(crate) fn extract_existing_dnsmasq_options(root: &Element) -> Result<HashSet<String>> {
    let mut options = HashSet::new();

    if let Some(dnsmasq) = find_descendant_ci(root, "dnsmasq") {
        for child in &dnsmasq.children {
            if let Some(opt) = child.as_element() {
                if opt.name.eq_ignore_ascii_case("dhcp_options") {
                    let opt_type = get_child_ci(opt, "type")
                        .and_then(|e| e.get_text())
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    if !opt_type.eq_ignore_ascii_case("set") {
                        continue;
                    }
                    let option = get_child_ci(opt, "option")
                        .and_then(|e| e.get_text())
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let option6 = get_child_ci(opt, "option6")
                        .and_then(|e| e.get_text())
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let iface = get_child_ci(opt, "interface")
                        .and_then(|e| e.get_text())
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let tag = get_child_ci(opt, "tag")
                        .and_then(|e| e.get_text())
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let set_tag = get_child_ci(opt, "set_tag")
                        .and_then(|e| e.get_text())
                        .map(|s| s.to_string())
                        .unwrap_or_default();

                    let key =
                        dnsmasq_option_key(&opt_type, &option, &option6, &iface, &tag, &set_tag);
                    options.insert(key);
                }
            }
        }
    }

    Ok(options)
}
