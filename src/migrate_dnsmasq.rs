use anyhow::{anyhow, Result};
use xmltree::{Element, XMLNode};

use crate::xml_helpers::{find_mut_descendant_ci, get_child_ci};
use crate::{IscStaticMap, IscStaticMapV6};

/// Create a dnsmasq host XML element from an ISC static mapping.
///
/// dnsmasq hosts are flat under `<dnsmasq><hosts>` with no subnet association.
pub fn create_dnsmasq_host_element(mapping: &IscStaticMap) -> Element {
    let mut host = Element::new("hosts");
    host.attributes
        .insert("uuid".to_string(), uuid::Uuid::new_v4().to_string());

    let mut hwaddr = Element::new("hwaddr");
    hwaddr.children.push(XMLNode::Text(mapping.mac.clone()));
    host.children.push(XMLNode::Element(hwaddr));

    let mut ip = Element::new("ip");
    ip.children.push(XMLNode::Text(mapping.ipaddr.clone()));
    host.children.push(XMLNode::Element(ip));

    // hostname
    let hostname_text = mapping
        .hostname
        .as_ref()
        .or(mapping.cid.as_ref())
        .cloned()
        .unwrap_or_default();
    let mut hostname = Element::new("host");
    hostname.children.push(XMLNode::Text(hostname_text));
    host.children.push(XMLNode::Element(hostname));

    // client_id
    if let Some(cid) = &mapping.cid {
        let mut client_id = Element::new("client_id");
        client_id.children.push(XMLNode::Text(cid.clone()));
        host.children.push(XMLNode::Element(client_id));
    }

    // description
    if let Some(d) = &mapping.descr {
        let mut descr = Element::new("descr");
        descr.children.push(XMLNode::Text(d.clone()));
        host.children.push(XMLNode::Element(descr));
    }

    // Defaults for fields dnsmasq expects
    for (tag, default) in [
        ("domain", ""),
        ("local", "0"),
        ("ignore", "0"),
        ("lease_time", ""),
        ("cnames", ""),
        ("set_tag", ""),
        ("comments", ""),
        ("aliases", ""),
    ] {
        let mut elem = Element::new(tag);
        elem.children.push(XMLNode::Text(default.to_string()));
        host.children.push(XMLNode::Element(elem));
    }

    host
}

/// Create a dnsmasq host XML element from an ISC DHCPv6 static mapping.
pub fn create_dnsmasq_host_element_v6(mapping: &IscStaticMapV6) -> Element {
    let mut host = Element::new("hosts");
    host.attributes
        .insert("uuid".to_string(), uuid::Uuid::new_v4().to_string());

    // hostname
    let hostname_text = mapping.hostname.clone().unwrap_or_default();
    let mut hostname = Element::new("host");
    hostname.children.push(XMLNode::Text(hostname_text));
    host.children.push(XMLNode::Element(hostname));

    // domain (best-effort: first entry from domain search list)
    let mut domain_elem = Element::new("domain");
    let domain_value = mapping
        .domain_search
        .as_deref()
        .map(first_domain)
        .unwrap_or_default();
    domain_elem.children.push(XMLNode::Text(domain_value));
    host.children.push(XMLNode::Element(domain_elem));

    let mut local = Element::new("local");
    local.children.push(XMLNode::Text("0".to_string()));
    host.children.push(XMLNode::Element(local));

    let mut ip = Element::new("ip");
    ip.children.push(XMLNode::Text(mapping.ipaddr.clone()));
    host.children.push(XMLNode::Element(ip));

    // client_id (DUID)
    let mut client_id = Element::new("client_id");
    client_id.children.push(XMLNode::Text(mapping.duid.clone()));
    host.children.push(XMLNode::Element(client_id));

    // hwaddr (not available for DHCPv6 mappings)
    let mut hwaddr = Element::new("hwaddr");
    hwaddr.children.push(XMLNode::Text("".to_string()));
    host.children.push(XMLNode::Element(hwaddr));

    // description
    if let Some(d) = &mapping.descr {
        let mut descr = Element::new("descr");
        descr.children.push(XMLNode::Text(d.clone()));
        host.children.push(XMLNode::Element(descr));
    }

    // Defaults for fields dnsmasq expects
    for (tag, default) in [
        ("lease_time", ""),
        ("cnames", ""),
        ("ignore", "0"),
        ("set_tag", ""),
        ("comments", ""),
        ("aliases", ""),
    ] {
        let mut elem = Element::new(tag);
        elem.children.push(XMLNode::Text(default.to_string()));
        host.children.push(XMLNode::Element(elem));
    }

    host
}

fn first_domain(domain_search: &str) -> String {
    domain_search
        .split(|c: char| c.is_whitespace() || c == ',')
        .find(|s| !s.is_empty())
        .unwrap_or("")
        .to_string()
}

/// Get the `<dnsmasq>` node, returning an error if it doesn't exist.
///
/// Unlike Kea, we don't need to navigate to a subnets container.
/// dnsmasq hosts are appended directly as `<hosts>` children of `<dnsmasq>`.
pub fn get_dnsmasq_node(root: &mut Element) -> Result<&mut Element> {
    // First check if dnsmasq exists (immutable check)
    if get_child_ci(root, "dnsmasq").is_none() {
        // Check one level deeper (opnsense > dnsmasq)
        let has_it = root
            .children
            .iter()
            .filter_map(|n| n.as_element())
            .any(|child| {
                child
                    .children
                    .iter()
                    .filter_map(|n| n.as_element())
                    .any(|grandchild| grandchild.name.eq_ignore_ascii_case("dnsmasq"))
            });
        if !has_it {
            return Err(anyhow!(
                "dnsmasq not configured in config.xml. Please configure dnsmasq first."
            ));
        }
    }

    find_mut_descendant_ci(root, "dnsmasq").ok_or_else(|| anyhow!("Failed to access dnsmasq node"))
}
