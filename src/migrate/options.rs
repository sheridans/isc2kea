use anyhow::{anyhow, Result};
use xmltree::{Element, XMLNode};

use crate::extract::{extract_interface_cidrs, extract_interface_cidrs_v6};
use crate::extract_dnsmasq::dnsmasq_option_key;
use crate::{IscDhcpOptionsV4, IscDhcpOptionsV6};

/// Apply ISC DHCP options into Kea option_data, per-interface.
pub(crate) fn apply_kea_options(
    root: &mut Element,
    options_v4: &[IscDhcpOptionsV4],
    options_v6: &[IscDhcpOptionsV6],
    force: bool,
) -> Result<()> {
    let iface_cidrs_v4 = extract_interface_cidrs(root)?;
    let iface_cidrs_v6 = extract_interface_cidrs_v6(root)?;

    let mut v4_by_cidr = std::collections::HashMap::new();
    for opt in options_v4 {
        if let Some(cidr) = iface_cidrs_v4.get(&opt.iface) {
            v4_by_cidr.insert(cidr.clone(), opt.clone());
        } else {
            eprintln!(
                "Warning: No interface CIDR found for DHCPv4 options (iface {}). Skipping.",
                opt.iface
            );
        }
    }

    let mut v6_by_cidr = std::collections::HashMap::new();
    for opt in options_v6 {
        if let Some(cidr) = iface_cidrs_v6.get(&opt.iface) {
            v6_by_cidr.insert(cidr.clone(), opt.clone());
        } else {
            eprintln!(
                "Warning: No interface CIDR found for DHCPv6 options (iface {}). Skipping.",
                opt.iface
            );
        }
    }

    // DHCPv4 options
    if let Some(kea) = crate::xml_helpers::find_mut_descendant_ci(root, "Kea") {
        if let Some(dhcp4) = crate::xml_helpers::find_mut_descendant_ci(kea, "dhcp4") {
            if let Some(subnets) = crate::xml_helpers::get_mut_child_ci(dhcp4, "subnets") {
                for subnet in subnets
                    .children
                    .iter_mut()
                    .filter_map(|n| n.as_mut_element())
                    .filter(|e| e.name.eq_ignore_ascii_case("subnet4"))
                {
                    let cidr = crate::xml_helpers::get_child_ci(subnet, "subnet")
                        .and_then(|e| e.get_text())
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let Some(opt) = v4_by_cidr.get(&cidr) else {
                        continue;
                    };

                    if crate::xml_helpers::get_mut_child_ci(subnet, "option_data").is_none() {
                        subnet
                            .children
                            .push(XMLNode::Element(Element::new("option_data")));
                    }

                    if let Some(auto) =
                        crate::xml_helpers::get_mut_child_ci(subnet, "option_data_autocollect")
                    {
                        auto.children.clear();
                        auto.children.push(XMLNode::Text("0".to_string()));
                    } else {
                        let mut auto = Element::new("option_data_autocollect");
                        auto.children.push(XMLNode::Text("0".to_string()));
                        subnet.children.push(XMLNode::Element(auto));
                    }

                    let option_data = crate::xml_helpers::get_mut_child_ci(subnet, "option_data")
                        .ok_or_else(|| anyhow!("Failed to access Kea option_data"))?;

                    set_option_value(
                        option_data,
                        "domain_name_servers",
                        join_list(&opt.dns_servers),
                        force,
                    );
                    set_option_value(option_data, "routers", opt.routers.clone(), force);
                    set_option_value(option_data, "domain_name", opt.domain_name.clone(), force);
                    set_option_value(
                        option_data,
                        "domain_search",
                        opt.domain_search.clone(),
                        force,
                    );
                    set_option_value(
                        option_data,
                        "ntp_servers",
                        join_list(&opt.ntp_servers),
                        force,
                    );
                }
            }
        }
    }

    // DHCPv6 options
    if let Some(kea) = crate::xml_helpers::find_mut_descendant_ci(root, "Kea") {
        if let Some(dhcp6) = crate::xml_helpers::find_mut_descendant_ci(kea, "dhcp6") {
            if let Some(subnets) = crate::xml_helpers::get_mut_child_ci(dhcp6, "subnets") {
                for subnet in subnets
                    .children
                    .iter_mut()
                    .filter_map(|n| n.as_mut_element())
                    .filter(|e| e.name.eq_ignore_ascii_case("subnet6"))
                {
                    let cidr = crate::xml_helpers::get_child_ci(subnet, "subnet")
                        .and_then(|e| e.get_text())
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    let Some(opt) = v6_by_cidr.get(&cidr) else {
                        continue;
                    };

                    if crate::xml_helpers::get_mut_child_ci(subnet, "option_data").is_none() {
                        subnet
                            .children
                            .push(XMLNode::Element(Element::new("option_data")));
                    }
                    let option_data = crate::xml_helpers::get_mut_child_ci(subnet, "option_data")
                        .ok_or_else(|| anyhow!("Failed to access Kea option_data"))?;

                    set_option_value(
                        option_data,
                        "dns_servers",
                        join_list(&opt.dns_servers),
                        force,
                    );
                    set_option_value(
                        option_data,
                        "domain_search",
                        opt.domain_search.clone(),
                        force,
                    );
                }
            }
        }
    }

    Ok(())
}

fn set_option_value(target: &mut Element, tag: &str, value: Option<String>, force: bool) {
    let Some(val) = value.filter(|v| !v.is_empty()) else {
        return;
    };
    let child = crate::xml_helpers::get_mut_child_ci(target, tag);
    match child {
        Some(elem) => {
            let existing = elem.get_text().map(|v| v.to_string()).unwrap_or_default();
            if !existing.is_empty() && !force {
                eprintln!(
                    "Warning: Kea option {} already set ({}). Skipping.",
                    tag, existing
                );
                return;
            }
            elem.children.clear();
            elem.children.push(XMLNode::Text(val));
        }
        None => {
            let mut elem = Element::new(tag);
            elem.children.push(XMLNode::Text(val));
            target.children.push(XMLNode::Element(elem));
        }
    }
}

fn join_list(values: &[String]) -> Option<String> {
    let filtered = dedupe_preserve_order(values.iter().filter(|v| !v.is_empty()));
    if filtered.is_empty() {
        None
    } else {
        Some(filtered.join(","))
    }
}

fn dedupe_preserve_order<'a, I>(values: I) -> Vec<String>
where
    I: Iterator<Item = &'a String>,
{
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for value in values {
        if seen.insert(value) {
            out.push(value.clone());
        }
    }
    out
}

#[derive(Debug, Clone)]
pub(crate) struct DnsmasqOptionSpec {
    pub(crate) iface: String,
    pub(crate) option: String,
    pub(crate) option6: String,
    pub(crate) value: String,
}

fn domain_search_csv(value: &str) -> Option<String> {
    let parts: Vec<String> = value
        .split(|c: char| c == ';' || c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(","))
    }
}

pub(crate) fn dnsmasq_option_specs_from_isc(
    options_v4: &[IscDhcpOptionsV4],
    options_v6: &[IscDhcpOptionsV6],
) -> Vec<DnsmasqOptionSpec> {
    let mut specs = Vec::new();

    for opt in options_v4 {
        if let Some(value) = join_list(&opt.dns_servers) {
            specs.push(DnsmasqOptionSpec {
                iface: opt.iface.clone(),
                option: "6".to_string(),
                option6: String::new(),
                value,
            });
        }
        if let Some(value) = opt.routers.clone().filter(|v| !v.is_empty()) {
            specs.push(DnsmasqOptionSpec {
                iface: opt.iface.clone(),
                option: "3".to_string(),
                option6: String::new(),
                value,
            });
        }
        if let Some(value) = opt.domain_name.clone().filter(|v| !v.is_empty()) {
            specs.push(DnsmasqOptionSpec {
                iface: opt.iface.clone(),
                option: "15".to_string(),
                option6: String::new(),
                value,
            });
        }
        if let Some(value) = opt.domain_search.as_deref().and_then(domain_search_csv) {
            specs.push(DnsmasqOptionSpec {
                iface: opt.iface.clone(),
                option: "119".to_string(),
                option6: String::new(),
                value,
            });
        }
        if let Some(value) = join_list(&opt.ntp_servers) {
            specs.push(DnsmasqOptionSpec {
                iface: opt.iface.clone(),
                option: "42".to_string(),
                option6: String::new(),
                value,
            });
        }
    }

    for opt in options_v6 {
        if let Some(value) = join_list(&opt.dns_servers) {
            specs.push(DnsmasqOptionSpec {
                iface: opt.iface.clone(),
                option: String::new(),
                option6: "23".to_string(),
                value,
            });
        }
        if let Some(value) = opt.domain_search.as_deref().and_then(domain_search_csv) {
            specs.push(DnsmasqOptionSpec {
                iface: opt.iface.clone(),
                option: String::new(),
                option6: "24".to_string(),
                value,
            });
        }
    }

    specs
}

pub(crate) fn dnsmasq_option_key_from_elem(elem: &Element) -> Option<String> {
    if !elem.name.eq_ignore_ascii_case("dhcp_options") {
        return None;
    }
    let opt_type = crate::xml_helpers::get_child_ci(elem, "type")
        .and_then(|e| e.get_text())
        .map(|s| s.to_string())
        .unwrap_or_default();
    if !opt_type.eq_ignore_ascii_case("set") {
        return None;
    }
    let option = crate::xml_helpers::get_child_ci(elem, "option")
        .and_then(|e| e.get_text())
        .map(|s| s.to_string())
        .unwrap_or_default();
    let option6 = crate::xml_helpers::get_child_ci(elem, "option6")
        .and_then(|e| e.get_text())
        .map(|s| s.to_string())
        .unwrap_or_default();
    let iface = crate::xml_helpers::get_child_ci(elem, "interface")
        .and_then(|e| e.get_text())
        .map(|s| s.to_string())
        .unwrap_or_default();
    let tag = crate::xml_helpers::get_child_ci(elem, "tag")
        .and_then(|e| e.get_text())
        .map(|s| s.to_string())
        .unwrap_or_default();
    let set_tag = crate::xml_helpers::get_child_ci(elem, "set_tag")
        .and_then(|e| e.get_text())
        .map(|s| s.to_string())
        .unwrap_or_default();
    Some(dnsmasq_option_key(
        &opt_type, &option, &option6, &iface, &tag, &set_tag,
    ))
}
