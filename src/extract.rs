use anyhow::Result;
use std::collections::HashSet;
use xmltree::Element;

use crate::xml_helpers::{find_descendant_ci, get_child_ci};
use crate::{
    IscDhcpOptionsV4, IscDhcpOptionsV6, IscRangeV4, IscRangeV6, IscStaticMap, IscStaticMapV6,
    Subnet, SubnetV6,
};
use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

/// Check if Kea DHCPv4 is configured (recursive search)
pub(crate) fn has_kea_dhcp4(root: &Element) -> bool {
    find_descendant_ci(root, "Kea")
        .and_then(|kea| find_descendant_ci(kea, "dhcp4"))
        .is_some()
}

/// Check if Kea DHCPv6 is configured (recursive search)
pub(crate) fn has_kea_dhcp6(root: &Element) -> bool {
    find_descendant_ci(root, "Kea")
        .and_then(|kea| find_descendant_ci(kea, "dhcp6"))
        .is_some()
}

/// Extract ISC static mappings from the XML tree
pub fn extract_isc_mappings(root: &Element) -> Result<Vec<IscStaticMap>> {
    let mut mappings = Vec::new();

    // Navigate to <dhcpd> (case-insensitive)
    if let Some(dhcpd) = get_child_ci(root, "dhcpd") {
        // Iterate over all interface nodes (lan, wan, opt1, etc.)
        for iface_node in dhcpd.children.iter() {
            if let Some(iface_elem) = iface_node.as_element() {
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

/// Extract Kea subnet4 entries from the XML tree
/// Supports two schema variations:
/// 1. <Kea><dhcp4><subnets><subnet4>... (standard OPNsense)
/// 2. <Kea><dhcp4><subnet4>... (fallback for plugin variations)
pub fn extract_kea_subnets(root: &Element) -> Result<Vec<Subnet>> {
    let mut subnets = Vec::new();

    // Navigate to <Kea>/<kea> (case-insensitive) -> <dhcp4>
    if let Some(kea) = find_descendant_ci(root, "Kea") {
        if let Some(dhcp4) = find_descendant_ci(kea, "dhcp4") {
            // Try standard path: <dhcp4><subnets><subnet4>
            let subnet_container = if let Some(subnets_node) = get_child_ci(dhcp4, "subnets") {
                Some(subnets_node)
            } else {
                // Fallback: <subnet4> directly under <dhcp4>
                Some(dhcp4)
            };

            if let Some(container) = subnet_container {
                for child in container.children.iter() {
                    if let Some(subnet4) = child.as_element() {
                        if subnet4.name.eq_ignore_ascii_case("subnet4") {
                            if let Some(uuid) = subnet4.attributes.get("uuid") {
                                if let Some(subnet_elem) = get_child_ci(subnet4, "subnet") {
                                    if let Some(cidr) = subnet_elem.get_text() {
                                        subnets.push(Subnet {
                                            uuid: uuid.to_string(),
                                            cidr: cidr.to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(subnets)
}

/// Extract Kea subnet6 entries from the XML tree
/// Supports <Kea><dhcp6><subnets><subnet6>
pub fn extract_kea_subnets_v6(root: &Element) -> Result<Vec<SubnetV6>> {
    let mut subnets = Vec::new();

    if let Some(kea) = find_descendant_ci(root, "Kea") {
        if let Some(dhcp6) = find_descendant_ci(kea, "dhcp6") {
            if let Some(subnets_node) = get_child_ci(dhcp6, "subnets") {
                for child in subnets_node.children.iter() {
                    if let Some(subnet6) = child.as_element() {
                        if subnet6.name.eq_ignore_ascii_case("subnet6") {
                            if let Some(uuid) = subnet6.attributes.get("uuid") {
                                if let Some(subnet_elem) = get_child_ci(subnet6, "subnet") {
                                    if let Some(cidr) = subnet_elem.get_text() {
                                        subnets.push(SubnetV6 {
                                            uuid: uuid.to_string(),
                                            cidr: cidr.to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(subnets)
}

/// Extract existing Kea reservation IP addresses for duplicate detection
pub fn extract_existing_reservation_ips(root: &Element) -> Result<HashSet<String>> {
    let mut ips = HashSet::new();

    // Navigate to <Kea>/<kea> (case-insensitive) -> <dhcp4> -> <reservations>
    if let Some(kea) = find_descendant_ci(root, "Kea") {
        if let Some(dhcp4) = find_descendant_ci(kea, "dhcp4") {
            if let Some(reservations) = find_descendant_ci(dhcp4, "reservations") {
                for child in reservations.children.iter() {
                    if let Some(reservation) = child.as_element() {
                        if reservation.name.eq_ignore_ascii_case("reservation") {
                            if let Some(ip_elem) = get_child_ci(reservation, "ip_address") {
                                if let Some(ip) = ip_elem.get_text() {
                                    ips.insert(ip.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(ips)
}

/// Extract existing Kea DHCPv6 reservation IP addresses for duplicate detection
pub fn extract_existing_reservation_ips_v6(root: &Element) -> Result<HashSet<String>> {
    let mut ips = HashSet::new();

    if let Some(kea) = find_descendant_ci(root, "Kea") {
        if let Some(dhcp6) = find_descendant_ci(kea, "dhcp6") {
            if let Some(reservations) = find_descendant_ci(dhcp6, "reservations") {
                for child in reservations.children.iter() {
                    if let Some(reservation) = child.as_element() {
                        if reservation.name.eq_ignore_ascii_case("reservation") {
                            if let Some(ip_elem) = get_child_ci(reservation, "ip_address") {
                                if let Some(ip) = ip_elem.get_text() {
                                    ips.insert(ip.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(ips)
}

/// Extract existing Kea DHCPv6 reservation DUIDs for duplicate detection
pub fn extract_existing_reservation_duids_v6(root: &Element) -> Result<HashSet<String>> {
    let mut duids = HashSet::new();

    if let Some(kea) = find_descendant_ci(root, "Kea") {
        if let Some(dhcp6) = find_descendant_ci(kea, "dhcp6") {
            if let Some(reservations) = find_descendant_ci(dhcp6, "reservations") {
                for child in reservations.children.iter() {
                    if let Some(reservation) = child.as_element() {
                        if reservation.name.eq_ignore_ascii_case("reservation") {
                            if let Some(duid_elem) = get_child_ci(reservation, "duid") {
                                if let Some(duid) = duid_elem.get_text() {
                                    duids.insert(duid.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(duids)
}
