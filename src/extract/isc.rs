use anyhow::Result;
use xmltree::Element;

use crate::xml_helpers::get_child_ci;
use crate::{
    IscDhcpOptionsV4, IscDhcpOptionsV6, IscRangeV4, IscRangeV6, IscStaticMap, IscStaticMapV6,
};

/// Extract ISC static mappings from the XML tree
pub fn extract_isc_mappings(root: &Element) -> Result<Vec<IscStaticMap>> {
    let mut mappings = Vec::new();

    // Navigate to <dhcpd> (case-insensitive)
    if let Some(dhcpd) = get_child_ci(root, "dhcpd") {
        // Iterate over all interface nodes (lan, wan, opt1, etc.)
        for iface_node in dhcpd.children.iter() {
            if let Some(iface_elem) = iface_node.as_element() {
                let iface_name = iface_elem.name.clone();
                // Look for staticmap children (case-insensitive)
                for child in iface_elem.children.iter() {
                    if let Some(staticmap) = child.as_element() {
                        if staticmap.name.eq_ignore_ascii_case("staticmap") {
                            let mac = get_child_ci(staticmap, "mac")
                                .and_then(|e| e.get_text())
                                .map(|s| s.to_string())
                                .unwrap_or_default();

                            let ipaddr = get_child_ci(staticmap, "ipaddr")
                                .and_then(|e| e.get_text())
                                .map(|s| s.to_string())
                                .unwrap_or_default();

                            // Skip entries without essential fields
                            if mac.is_empty() || ipaddr.is_empty() {
                                continue;
                            }

                            let hostname = get_child_ci(staticmap, "hostname")
                                .and_then(|e| e.get_text())
                                .map(|s| s.to_string());

                            let cid = get_child_ci(staticmap, "cid")
                                .and_then(|e| e.get_text())
                                .map(|s| s.to_string());

                            let descr = get_child_ci(staticmap, "descr")
                                .and_then(|e| e.get_text())
                                .map(|s| s.to_string());

                            mappings.push(IscStaticMap {
                                iface: iface_name.clone(),
                                mac,
                                ipaddr,
                                hostname,
                                cid,
                                descr,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(mappings)
}

/// Extract ISC DHCPv6 static mappings from the XML tree
pub fn extract_isc_mappings_v6(root: &Element) -> Result<Vec<IscStaticMapV6>> {
    let mut mappings = Vec::new();

    // Navigate to <dhcpdv6> (case-insensitive)
    if let Some(dhcpdv6) = get_child_ci(root, "dhcpdv6") {
        // Iterate over all interface nodes (lan, wan, opt1, etc.)
        for iface_node in dhcpdv6.children.iter() {
            if let Some(iface_elem) = iface_node.as_element() {
                let iface_name = iface_elem.name.clone();
                // Look for staticmap children (case-insensitive)
                for child in iface_elem.children.iter() {
                    if let Some(staticmap) = child.as_element() {
                        if staticmap.name.eq_ignore_ascii_case("staticmap") {
                            let duid = get_child_ci(staticmap, "duid")
                                .and_then(|e| e.get_text())
                                .map(|s| s.to_string())
                                .unwrap_or_default();

                            let ipaddr = get_child_ci(staticmap, "ipaddrv6")
                                .and_then(|e| e.get_text())
                                .map(|s| s.to_string())
                                .unwrap_or_default();

                            // Skip entries without essential fields
                            if duid.is_empty() || ipaddr.is_empty() {
                                continue;
                            }

                            let hostname = get_child_ci(staticmap, "hostname")
                                .and_then(|e| e.get_text())
                                .map(|s| s.to_string());

                            let descr = get_child_ci(staticmap, "descr")
                                .and_then(|e| e.get_text())
                                .map(|s| s.to_string());

                            let domain_search = get_child_ci(staticmap, "domainsearchlist")
                                .and_then(|e| e.get_text())
                                .map(|s| s.to_string());

                            mappings.push(IscStaticMapV6 {
                                iface: iface_name.clone(),
                                duid,
                                ipaddr,
                                hostname,
                                descr,
                                domain_search,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(mappings)
}

/// Extract ISC DHCPv4 options per interface
pub fn extract_isc_options_v4(root: &Element) -> Result<Vec<IscDhcpOptionsV4>> {
    let mut options = Vec::new();

    if let Some(dhcpd) = get_child_ci(root, "dhcpd") {
        for iface_node in dhcpd.children.iter() {
            if let Some(iface_elem) = iface_node.as_element() {
                let iface_name = iface_elem.name.clone();
                let mut dns_servers = Vec::new();
                let mut ntp_servers = Vec::new();
                let mut routers = None;
                let mut domain_name = None;
                let mut domain_search = None;

                for child in iface_elem.children.iter().filter_map(|c| c.as_element()) {
                    if child.name.eq_ignore_ascii_case("dnsserver") {
                        if let Some(val) = child.get_text() {
                            let v = val.to_string();
                            if !v.is_empty() {
                                dns_servers.push(v);
                            }
                        }
                    }
                    if child.name.eq_ignore_ascii_case("ntpserver") {
                        if let Some(val) = child.get_text() {
                            let v = val.to_string();
                            if !v.is_empty() {
                                ntp_servers.push(v);
                            }
                        }
                    }
                    if child.name.eq_ignore_ascii_case("gateway") {
                        routers = child
                            .get_text()
                            .map(|v| v.to_string())
                            .filter(|v| !v.is_empty());
                    }
                    if child.name.eq_ignore_ascii_case("domain") {
                        domain_name = child
                            .get_text()
                            .map(|v| v.to_string())
                            .filter(|v| !v.is_empty());
                    }
                    if child.name.eq_ignore_ascii_case("domainsearchlist") {
                        domain_search = child
                            .get_text()
                            .map(|v| v.to_string())
                            .filter(|v| !v.is_empty());
                    }
                }

                if !dns_servers.is_empty()
                    || !ntp_servers.is_empty()
                    || routers.is_some()
                    || domain_name.is_some()
                    || domain_search.is_some()
                {
                    options.push(IscDhcpOptionsV4 {
                        iface: iface_name,
                        dns_servers,
                        routers,
                        domain_name,
                        domain_search: domain_search.map(normalize_domain_search),
                        ntp_servers,
                    });
                }
            }
        }
    }

    Ok(options)
}

/// Extract ISC DHCPv6 options per interface
pub fn extract_isc_options_v6(root: &Element) -> Result<Vec<IscDhcpOptionsV6>> {
    let mut options = Vec::new();

    if let Some(dhcpdv6) = get_child_ci(root, "dhcpdv6") {
        for iface_node in dhcpdv6.children.iter() {
            if let Some(iface_elem) = iface_node.as_element() {
                let iface_name = iface_elem.name.clone();
                let mut dns_servers = Vec::new();
                let mut domain_search = None;

                for child in iface_elem.children.iter().filter_map(|c| c.as_element()) {
                    if child.name.eq_ignore_ascii_case("dnsserver") {
                        if let Some(val) = child.get_text() {
                            let v = val.to_string();
                            if !v.is_empty() {
                                dns_servers.push(v);
                            }
                        }
                    }
                    if child.name.eq_ignore_ascii_case("domainsearchlist") {
                        domain_search = child
                            .get_text()
                            .map(|v| v.to_string())
                            .filter(|v| !v.is_empty());
                    }
                }

                if !dns_servers.is_empty() || domain_search.is_some() {
                    options.push(IscDhcpOptionsV6 {
                        iface: iface_name,
                        dns_servers,
                        domain_search: domain_search.map(normalize_domain_search),
                    });
                }
            }
        }
    }

    Ok(options)
}

fn normalize_domain_search(raw: String) -> String {
    raw.split(|c: char| c == ';' || c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract ISC DHCPv4 ranges from the XML tree
pub fn extract_isc_ranges(root: &Element) -> Result<Vec<IscRangeV4>> {
    let mut ranges = Vec::new();

    if let Some(dhcpd) = get_child_ci(root, "dhcpd") {
        for iface_node in dhcpd.children.iter() {
            if let Some(iface_elem) = iface_node.as_element() {
                let iface_name = iface_elem.name.clone();
                for child in iface_elem.children.iter() {
                    if let Some(range) = child.as_element() {
                        if range.name.eq_ignore_ascii_case("range") {
                            let from = get_child_ci(range, "from")
                                .and_then(|e| e.get_text())
                                .map(|s| s.to_string())
                                .unwrap_or_default();
                            let to = get_child_ci(range, "to")
                                .and_then(|e| e.get_text())
                                .map(|s| s.to_string())
                                .unwrap_or_default();
                            if from.is_empty() || to.is_empty() {
                                continue;
                            }
                            ranges.push(IscRangeV4 {
                                iface: iface_name.clone(),
                                from,
                                to,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(ranges)
}

/// Extract ISC DHCPv6 ranges from the XML tree
pub fn extract_isc_ranges_v6(root: &Element) -> Result<Vec<IscRangeV6>> {
    let mut ranges = Vec::new();

    if let Some(dhcpdv6) = get_child_ci(root, "dhcpdv6") {
        for iface_node in dhcpdv6.children.iter() {
            if let Some(iface_elem) = iface_node.as_element() {
                let iface_name = iface_elem.name.clone();
                for child in iface_elem.children.iter() {
                    if let Some(range) = child.as_element() {
                        if range.name.eq_ignore_ascii_case("range") {
                            let from = get_child_ci(range, "from")
                                .and_then(|e| e.get_text())
                                .map(|s| s.to_string())
                                .unwrap_or_default();
                            let to = get_child_ci(range, "to")
                                .and_then(|e| e.get_text())
                                .map(|s| s.to_string())
                                .unwrap_or_default();
                            if from.is_empty() || to.is_empty() {
                                continue;
                            }
                            ranges.push(IscRangeV6 {
                                iface: iface_name.clone(),
                                from,
                                to,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(ranges)
}
