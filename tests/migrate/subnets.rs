use super::common::*;
use isc2kea::{convert_config, scan_config, MigrationOptions};
use std::io::Cursor;
use xmltree::Element;

#[test]
fn test_create_subnets_kea_v4() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_KEA_V4);
    let mut output = Vec::new();
    let options = MigrationOptions {
        create_subnets: true,
        ..Default::default()
    };

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let kea = root.get_child("Kea").expect("Should have Kea node");
    let dhcp4 = kea.get_child("dhcp4").expect("Should have dhcp4 node");
    let subnets = dhcp4
        .get_child("subnets")
        .expect("Should have subnets node");

    let subnet4 = subnets
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .find(|e| e.name == "subnet4")
        .expect("Should have subnet4");

    let subnet_cidr = subnet4
        .get_child("subnet")
        .and_then(|e| e.get_text())
        .expect("Should have subnet");
    assert_eq!(subnet_cidr, "10.22.1.0/24");

    let pools = subnet4.get_child("pools").expect("Should have pools");
    let pool_value = pools.get_text().expect("Should have pool value");
    assert_eq!(pool_value, "10.22.1.100-10.22.1.200");
}

#[test]
fn test_create_subnets_kea_v6() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_KEA_V6);
    let mut output = Vec::new();
    let options = MigrationOptions {
        create_subnets: true,
        ..Default::default()
    };

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let kea = root.get_child("Kea").expect("Should have Kea node");
    let dhcp6 = kea.get_child("dhcp6").expect("Should have dhcp6 node");
    let subnets = dhcp6
        .get_child("subnets")
        .expect("Should have subnets node");

    let subnet6 = subnets
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .find(|e| e.name == "subnet6")
        .expect("Should have subnet6");

    let subnet_cidr = subnet6
        .get_child("subnet")
        .and_then(|e| e.get_text())
        .expect("Should have subnet");
    assert_eq!(subnet_cidr, "fd00:1234:5678:1::/64");

    let pools = subnet6.get_child("pools").expect("Should have pools");
    let pool_value = pools.get_text().expect("Should have pool value");
    assert_eq!(pool_value, "fd00:1234:5678:1::100-fd00:1234:5678:1::200");
}

#[test]
fn test_create_subnets_dnsmasq_v4() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_DNSMASQ_V4);
    let mut output = Vec::new();
    let options = dnsmasq_options_create_subnets();

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let dnsmasq = find_descendant_ci(&root, "dnsmasq").expect("Should have dnsmasq node");

    let range = dnsmasq
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .find(|e| e.name == "dhcp_ranges")
        .expect("Should have dhcp_ranges");

    let iface = range
        .get_child("interface")
        .and_then(|e| e.get_text())
        .expect("Should have interface");
    assert_eq!(iface, "opt1");

    let start = range
        .get_child("start_addr")
        .and_then(|e| e.get_text())
        .expect("Should have start_addr");
    assert_eq!(start, "10.22.1.100");

    let end = range
        .get_child("end_addr")
        .and_then(|e| e.get_text())
        .expect("Should have end_addr");
    assert_eq!(end, "10.22.1.200");

    let mask = range
        .get_child("subnet_mask")
        .and_then(|e| e.get_text())
        .expect("Should have subnet_mask");
    assert_eq!(mask, "255.255.255.0");
}

#[test]
fn test_create_subnets_dnsmasq_v6() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_DNSMASQ_V6);
    let mut output = Vec::new();
    let options = dnsmasq_options_create_subnets();

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let dnsmasq = find_descendant_ci(&root, "dnsmasq").expect("Should have dnsmasq node");

    let range = dnsmasq
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .find(|e| e.name == "dhcp_ranges")
        .expect("Should have dhcp_ranges");

    let iface = range
        .get_child("interface")
        .and_then(|e| e.get_text())
        .expect("Should have interface");
    assert_eq!(iface, "lan");

    let start = range
        .get_child("start_addr")
        .and_then(|e| e.get_text())
        .expect("Should have start_addr");
    assert_eq!(start, "fd00:1234:5678:1::100");

    let end = range
        .get_child("end_addr")
        .and_then(|e| e.get_text())
        .expect("Should have end_addr");
    assert_eq!(end, "fd00:1234:5678:1::200");

    let prefix_len = range
        .get_child("prefix_len")
        .and_then(|e| e.get_text())
        .expect("Should have prefix_len");
    assert_eq!(prefix_len, "64");
}

#[test]
fn test_create_subnets_kea_existing_skip() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_KEA_V4_EXISTING);
    let mut output = Vec::new();
    let options = MigrationOptions {
        create_subnets: true,
        ..Default::default()
    };

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let kea = root.get_child("Kea").expect("Should have Kea node");
    let dhcp4 = kea.get_child("dhcp4").expect("Should have dhcp4 node");
    let subnets = dhcp4
        .get_child("subnets")
        .expect("Should have subnets node");

    let subnet4 = subnets
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .find(|e| e.name == "subnet4")
        .expect("Should have subnet4");

    let pools = subnet4.get_child("pools").expect("Should have pools");
    let pool_value = pools.get_text().expect("Should have pool value");
    assert_eq!(pool_value, "10.22.1.50-10.22.1.60");
}

#[test]
fn test_create_subnets_kea_existing_force() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_KEA_V4_EXISTING);
    let mut output = Vec::new();
    let options = MigrationOptions {
        create_subnets: true,
        force_subnets: true,
        ..Default::default()
    };

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let kea = root.get_child("Kea").expect("Should have Kea node");
    let dhcp4 = kea.get_child("dhcp4").expect("Should have dhcp4 node");
    let subnets = dhcp4
        .get_child("subnets")
        .expect("Should have subnets node");

    let subnet4 = subnets
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .find(|e| e.name == "subnet4")
        .expect("Should have subnet4");

    let pools = subnet4.get_child("pools").expect("Should have pools");
    let pool_value = pools.get_text().expect("Should have pool value");
    assert_eq!(pool_value, "10.22.1.100-10.22.1.200");
}

#[test]
fn test_create_subnets_dnsmasq_existing_skip() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_DNSMASQ_V4_EXISTING);
    let mut output = Vec::new();
    let options = dnsmasq_options_create_subnets();

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let dnsmasq = find_descendant_ci(&root, "dnsmasq").expect("Should have dnsmasq node");

    let ranges: Vec<&Element> = dnsmasq
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .filter(|e| e.name == "dhcp_ranges")
        .collect();
    assert_eq!(ranges.len(), 1, "Should keep existing range only");

    let domain_type = ranges[0]
        .get_child("domain_type")
        .and_then(|e| e.get_text())
        .expect("Should have domain_type");
    assert_eq!(domain_type, "old");
}

#[test]
fn test_create_subnets_dnsmasq_existing_force() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_DNSMASQ_V4_EXISTING);
    let mut output = Vec::new();
    let mut options = dnsmasq_options_create_subnets();
    options.force_subnets = true;

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let dnsmasq = find_descendant_ci(&root, "dnsmasq").expect("Should have dnsmasq node");

    let ranges: Vec<&Element> = dnsmasq
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .filter(|e| e.name == "dhcp_ranges")
        .collect();
    assert_eq!(ranges.len(), 1, "Should replace existing range");

    let domain_type = ranges[0]
        .get_child("domain_type")
        .and_then(|e| e.get_text())
        .expect("Should have domain_type");
    assert_eq!(domain_type, "range");
}

#[test]
fn test_create_subnets_range_outside_cidr_errors() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_RANGE_OUTSIDE_CIDR);
    let mut output = Vec::new();
    let options = MigrationOptions {
        create_subnets: true,
        ..Default::default()
    };

    let err = convert_config(input, &mut output, &options)
        .expect_err("convert should fail for out-of-subnet range");
    assert!(err
        .to_string()
        .contains("not contained within interface subnet"));
}

#[test]
fn test_create_subnets_missing_interface_errors() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_MISSING_INTERFACE);
    let mut output = Vec::new();
    let options = MigrationOptions {
        create_subnets: true,
        ..Default::default()
    };

    let err = convert_config(input, &mut output, &options)
        .expect_err("convert should fail when interface CIDR is missing");
    assert!(err
        .to_string()
        .contains("No interface CIDR found for DHCPv4 interface"));
}

#[test]
fn test_create_subnets_dhcp_interface_errors() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_DHCP_INTERFACE);
    let mut output = Vec::new();
    let options = MigrationOptions {
        create_subnets: true,
        ..Default::default()
    };

    let err = convert_config(input, &mut output, &options)
        .expect_err("convert should fail for DHCP interface");
    assert!(err
        .to_string()
        .contains("No interface CIDR found for DHCPv4 interface"));
}

#[test]
fn test_create_subnets_track6_interface_errors() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_TRACK6_INTERFACE);
    let mut output = Vec::new();
    let options = MigrationOptions {
        create_subnets: true,
        ..Default::default()
    };

    let err = convert_config(input, &mut output, &options)
        .expect_err("convert should fail for track6 interface");
    assert!(err
        .to_string()
        .contains("No interface CIDR found for DHCPv6 interface"));
}

#[test]
fn test_scan_create_subnets_kea_no_mutation() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_KEA_V4);
    let options = MigrationOptions {
        create_subnets: true,
        ..Default::default()
    };

    let stats = scan_config(input, &options).expect("scan should succeed");
    assert_eq!(stats.target_subnets_found, 0);
    assert_eq!(stats.target_subnets_v6_found, 0);
}

#[test]
fn test_scan_create_subnets_dnsmasq_no_mutation() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_DNSMASQ_V4);
    let options = dnsmasq_options_create_subnets();

    let stats = scan_config(input, &options).expect("scan should succeed");
    assert_eq!(stats.target_subnets_found, 0);
    assert_eq!(stats.target_subnets_v6_found, 0);
}

#[test]
fn test_create_subnets_multiple_ranges_v4() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_MULTI_RANGE_V4);
    let mut output = Vec::new();
    let options = MigrationOptions {
        create_subnets: true,
        ..Default::default()
    };

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let kea = root.get_child("Kea").expect("Should have Kea node");
    let dhcp4 = kea.get_child("dhcp4").expect("Should have dhcp4 node");
    let subnets = dhcp4
        .get_child("subnets")
        .expect("Should have subnets node");

    let subnet4 = subnets
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .find(|e| e.name == "subnet4")
        .expect("Should have subnet4");

    let pools = subnet4.get_child("pools").expect("Should have pools");
    let pool_value = pools
        .get_text()
        .expect("Should have pool value")
        .to_string();
    let pool_parts: Vec<&str> = pool_value.split(',').collect();
    assert_eq!(pool_parts.len(), 2);
    assert!(pool_parts.contains(&"10.22.1.10-10.22.1.20"));
    assert!(pool_parts.contains(&"10.22.1.100-10.22.1.200"));
}

#[test]
fn test_create_subnets_multiple_ranges_v6() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_MULTI_RANGE_V6);
    let mut output = Vec::new();
    let options = MigrationOptions {
        create_subnets: true,
        ..Default::default()
    };

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let kea = root.get_child("Kea").expect("Should have Kea node");
    let dhcp6 = kea.get_child("dhcp6").expect("Should have dhcp6 node");
    let subnets = dhcp6
        .get_child("subnets")
        .expect("Should have subnets node");

    let subnet6 = subnets
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .find(|e| e.name == "subnet6")
        .expect("Should have subnet6");

    let pools = subnet6.get_child("pools").expect("Should have pools");
    let pool_value = pools
        .get_text()
        .expect("Should have pool value")
        .to_string();
    let pool_parts: Vec<&str> = pool_value.split(',').collect();
    assert_eq!(pool_parts.len(), 2);
    assert!(pool_parts.contains(&"fd00:1234:5678:1::10-fd00:1234:5678:1::20"));
    assert!(pool_parts.contains(&"fd00:1234:5678:1::100-fd00:1234:5678:1::200"));
}

#[test]
fn test_create_subnets_kea_adds_interfaces_v4() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_KEA_V4);
    let mut output = Vec::new();
    let options = MigrationOptions {
        create_subnets: true,
        ..Default::default()
    };

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let kea = root.get_child("Kea").expect("Should have Kea node");
    let dhcp4 = kea.get_child("dhcp4").expect("Should have dhcp4 node");
    let general = dhcp4
        .get_child("general")
        .expect("Should have general node");
    let interfaces = general
        .get_child("interfaces")
        .expect("Should have interfaces");
    let iface_value = interfaces.get_text().expect("Should have interface value");
    assert_eq!(iface_value, "opt1");
}

#[test]
fn test_create_subnets_kea_adds_interfaces_v6() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_KEA_V6);
    let mut output = Vec::new();
    let options = MigrationOptions {
        create_subnets: true,
        ..Default::default()
    };

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let kea = root.get_child("Kea").expect("Should have Kea node");
    let dhcp6 = kea.get_child("dhcp6").expect("Should have dhcp6 node");

    // Check general interfaces
    let general = dhcp6
        .get_child("general")
        .expect("Should have general node");
    let interfaces = general
        .get_child("interfaces")
        .expect("Should have interfaces");
    let iface_value = interfaces.get_text().expect("Should have interface value");
    assert_eq!(iface_value, "lan");

    // Check per-subnet interface
    let subnets = dhcp6
        .get_child("subnets")
        .expect("Should have subnets node");
    let subnet6 = subnets
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .find(|e| e.name == "subnet6")
        .expect("Should have subnet6");
    let subnet_iface = subnet6
        .get_child("interface")
        .expect("Should have interface in subnet6");
    let subnet_iface_value = subnet_iface
        .get_text()
        .expect("Should have interface value in subnet6");
    assert_eq!(subnet_iface_value, "lan");
}

#[test]
fn test_create_subnets_kea_preserves_existing_interfaces() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_KEA_V4_EXISTING_INTERFACES);
    let mut output = Vec::new();
    let options = MigrationOptions {
        create_subnets: true,
        ..Default::default()
    };

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let kea = root.get_child("Kea").expect("Should have Kea node");
    let dhcp4 = kea.get_child("dhcp4").expect("Should have dhcp4 node");
    let general = dhcp4
        .get_child("general")
        .expect("Should have general node");
    let interfaces = general
        .get_child("interfaces")
        .expect("Should have interfaces");
    let iface_value = interfaces
        .get_text()
        .expect("Should have interface value")
        .to_string();

    // Should have both existing (opt1) and new (opt2) interfaces
    let iface_parts: Vec<&str> = iface_value.split(',').collect();
    assert_eq!(iface_parts.len(), 2);
    assert!(iface_parts.contains(&"opt1"));
    assert!(iface_parts.contains(&"opt2"));
}

#[test]
fn test_create_subnets_dnsmasq_adds_interfaces() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_DNSMASQ_V4);
    let mut output = Vec::new();
    let options = dnsmasq_options_create_subnets();

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let dnsmasq = find_descendant_ci(&root, "dnsmasq").expect("Should have dnsmasq node");
    let interface = dnsmasq
        .get_child("interface")
        .expect("Should have interface");
    let iface_value = interface.get_text().expect("Should have interface value");
    assert_eq!(iface_value, "opt1");
}

#[test]
fn test_create_subnets_dnsmasq_adds_interfaces_v6() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_DNSMASQ_V6);
    let mut output = Vec::new();
    let options = dnsmasq_options_create_subnets();

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let dnsmasq = find_descendant_ci(&root, "dnsmasq").expect("Should have dnsmasq node");
    let interface = dnsmasq
        .get_child("interface")
        .expect("Should have interface");
    let iface_value = interface.get_text().expect("Should have interface value");
    assert_eq!(iface_value, "lan");
}

#[test]
fn test_create_subnets_dnsmasq_preserves_existing_interfaces() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_DNSMASQ_V4_EXISTING_INTERFACES);
    let mut output = Vec::new();
    let options = dnsmasq_options_create_subnets();

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let dnsmasq = find_descendant_ci(&root, "dnsmasq").expect("Should have dnsmasq node");
    let interface = dnsmasq
        .get_child("interface")
        .expect("Should have interface");
    let iface_value = interface
        .get_text()
        .expect("Should have interface value")
        .to_string();

    // Should have both existing (opt1) and new (opt2) interfaces
    let iface_parts: Vec<&str> = iface_value.split(',').collect();
    assert_eq!(iface_parts.len(), 2);
    assert!(iface_parts.contains(&"opt1"));
    assert!(iface_parts.contains(&"opt2"));
}
