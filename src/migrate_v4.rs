use anyhow::{anyhow, Result};
use xmltree::{Element, XMLNode};

use crate::xml_helpers::{find_mut_descendant_ci, get_child_ci, get_mut_child_ci};
use crate::{IscStaticMap, MigrationError};

/// Create a reservation XML element from an ISC mapping
pub fn create_reservation_element(mapping: &IscStaticMap, subnet_uuid: &str) -> Element {
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

    let mut hw_elem = Element::new("hw_address");
    hw_elem.children.push(XMLNode::Text(mapping.mac.clone()));
    reservation.children.push(XMLNode::Element(hw_elem));

    // hostname (prefer hostname over cid)
    if let Some(hostname) = &mapping.hostname {
        let mut hostname_elem = Element::new("hostname");
        hostname_elem.children.push(XMLNode::Text(hostname.clone()));
        reservation.children.push(XMLNode::Element(hostname_elem));
    } else if let Some(cid) = &mapping.cid {
        let mut hostname_elem = Element::new("hostname");
        hostname_elem.children.push(XMLNode::Text(cid.clone()));
        reservation.children.push(XMLNode::Element(hostname_elem));
    }

    if let Some(descr) = &mapping.descr {
        let mut descr_elem = Element::new("description");
        descr_elem.children.push(XMLNode::Text(descr.clone()));
        reservation.children.push(XMLNode::Element(descr_elem));
    }

    reservation
}

/// Get the <Kea>/<kea><dhcp4><reservations> node (case-insensitive)
/// Fails if Kea or dhcp4 sections don't exist (don't auto-create them)
/// Creates <reservations> if it doesn't exist but dhcp4 does
pub fn get_reservations_node(root: &mut Element) -> Result<&mut Element> {
    // Check <Kea>/<kea> exists (case-insensitive)
    let kea = find_mut_descendant_ci(root, "Kea").ok_or(MigrationError::KeaNotConfigured)?;

    // Check <dhcp4> exists
    let dhcp4 = find_mut_descendant_ci(kea, "dhcp4").ok_or(MigrationError::KeaNotConfigured)?;

    // Create <reservations> if it doesn't exist (this is safe - just adding reservation container)
    if get_child_ci(dhcp4, "reservations").is_none() {
        let reservations = Element::new("reservations");
        dhcp4.children.push(XMLNode::Element(reservations));
    }

    // Should always exist now after creation above
    get_mut_child_ci(dhcp4, "reservations")
        .ok_or_else(|| anyhow!("Failed to access reservations node after creating it"))
}
