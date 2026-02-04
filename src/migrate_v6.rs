use anyhow::{anyhow, Result};
use xmltree::{Element, XMLNode};

use crate::xml_helpers::{find_mut_descendant_ci, get_child_ci, get_mut_child_ci};
use crate::{IscStaticMapV6, MigrationError};

/// Create a DHCPv6 reservation XML element from an ISC mapping
pub fn create_reservation_element_v6(mapping: &IscStaticMapV6, subnet_uuid: &str) -> Element {
    let mut reservation = Element::new("reservation");
    reservation
        .attributes
        .insert("uuid".to_string(), uuid::Uuid::new_v4().to_string());

    let mut subnet_elem = Element::new("subnet");
    subnet_elem
        .children
        .push(XMLNode::Text(subnet_uuid.to_string()));
    reservation.children.push(XMLNode::Element(subnet_elem));

    let mut ip_elem = Element::new("ip_address");
    ip_elem.children.push(XMLNode::Text(mapping.ipaddr.clone()));
    reservation.children.push(XMLNode::Element(ip_elem));

    let mut duid_elem = Element::new("duid");
    duid_elem.children.push(XMLNode::Text(mapping.duid.clone()));
    reservation.children.push(XMLNode::Element(duid_elem));

    if let Some(hostname) = &mapping.hostname {
        let mut hostname_elem = Element::new("hostname");
        hostname_elem.children.push(XMLNode::Text(hostname.clone()));
        reservation.children.push(XMLNode::Element(hostname_elem));
    }

    if let Some(domain_search) = &mapping.domain_search {
        let mut domain_elem = Element::new("domain_search");
        domain_elem
            .children
            .push(XMLNode::Text(domain_search.clone()));
        reservation.children.push(XMLNode::Element(domain_elem));
    }

    if let Some(descr) = &mapping.descr {
        let mut descr_elem = Element::new("description");
        descr_elem.children.push(XMLNode::Text(descr.clone()));
        reservation.children.push(XMLNode::Element(descr_elem));
    }

    reservation
}

/// Get the <Kea>/<kea><dhcp6><reservations> node (case-insensitive)
/// Fails if Kea or dhcp6 sections don't exist (don't auto-create them)
/// Creates <reservations> if it doesn't exist but dhcp6 does
pub fn get_reservations_node_v6(root: &mut Element) -> Result<&mut Element> {
    let kea =
        find_mut_descendant_ci(root, "Kea").ok_or(MigrationError::BackendV6NotConfigured {
            backend: "Kea".into(),
        })?;
    let dhcp6 =
        find_mut_descendant_ci(kea, "dhcp6").ok_or(MigrationError::BackendV6NotConfigured {
            backend: "Kea".into(),
        })?;

    if get_child_ci(dhcp6, "reservations").is_none() {
        let reservations = Element::new("reservations");
        dhcp6.children.push(XMLNode::Element(reservations));
    }

    get_mut_child_ci(dhcp6, "reservations")
        .ok_or_else(|| anyhow!("Failed to access DHCPv6 reservations node after creating it"))
}
