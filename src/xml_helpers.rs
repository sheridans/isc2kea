use xmltree::Element;

fn name_matches(raw_name: &str, target: &str) -> bool {
    if raw_name.eq_ignore_ascii_case(target) {
        return true;
    }

    raw_name
        .rsplit_once(':')
        .map(|(_, suffix)| suffix.eq_ignore_ascii_case(target))
        .unwrap_or(false)
}

/// Get child element by name (case-insensitive)
pub(crate) fn get_child_ci<'a>(el: &'a Element, name: &str) -> Option<&'a Element> {
    el.children
        .iter()
        .filter_map(|n| n.as_element())
        .find(|c| name_matches(&c.name, name))
}

/// Get mutable child element by name (case-insensitive)
pub(crate) fn get_mut_child_ci<'a>(el: &'a mut Element, name: &str) -> Option<&'a mut Element> {
    el.children
        .iter_mut()
        .filter_map(|n| n.as_mut_element())
        .find(|c| name_matches(&c.name, name))
}

/// Find descendant element by name (case-insensitive)
pub(crate) fn find_descendant_ci<'a>(el: &'a Element, name: &str) -> Option<&'a Element> {
    for child in el.children.iter().filter_map(|n| n.as_element()) {
        if name_matches(&child.name, name) {
            return Some(child);
        }
        if let Some(found) = find_descendant_ci(child, name) {
            return Some(found);
        }
    }
    None
}

/// Find mutable descendant element by name (case-insensitive)
pub(crate) fn find_mut_descendant_ci<'a>(
    el: &'a mut Element,
    name: &str,
) -> Option<&'a mut Element> {
    for child in el.children.iter_mut().filter_map(|n| n.as_mut_element()) {
        if name_matches(&child.name, name) {
            return Some(child);
        }
        if let Some(found) = find_mut_descendant_ci(child, name) {
            return Some(found);
        }
    }
    None
}

/// Check if Kea DHCPv4 is configured (recursive search)
pub(crate) fn has_kea_dhcp4(root: &Element) -> bool {
    find_descendant_ci(root, "Kea")
        .and_then(|kea| find_descendant_ci(kea, "dhcp4"))
        .is_some()
}

pub(crate) fn has_kea_dhcp6(root: &Element) -> bool {
    find_descendant_ci(root, "Kea")
        .and_then(|kea| find_descendant_ci(kea, "dhcp6"))
        .is_some()
}
