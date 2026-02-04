use super::common::*;
use isc2kea::{convert_config, MigrationOptions};
use std::io::Cursor;
use xmltree::Element;

#[test]
fn test_create_options_kea_v4() {
    let input = Cursor::new(TEST_CREATE_OPTIONS_KEA_V4);
    let mut output = Vec::new();
    let options = MigrationOptions {
        create_options: true,
        ..Default::default()
    };

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let kea = root.get_child("Kea").expect("Should have Kea node");
    let dhcp4 = kea.get_child("dhcp4").expect("Should have dhcp4 node");
    let subnet4 = dhcp4
        .get_child("subnets")
        .and_then(|s| s.get_child("subnet4"))
        .expect("Should have subnet4");
    let option_data = subnet4
        .get_child("option_data")
        .expect("Should have option_data");

    let dns = option_data
        .get_child("domain_name_servers")
        .and_then(|e| e.get_text())
        .expect("Should have domain_name_servers");
    assert_eq!(dns, "8.8.8.8,1.1.1.1");

    let routers = option_data
        .get_child("routers")
        .and_then(|e| e.get_text())
        .expect("Should have routers");
    assert_eq!(routers, "10.22.1.1");

    let domain = option_data
        .get_child("domain_name")
        .and_then(|e| e.get_text())
        .expect("Should have domain_name");
    assert_eq!(domain, "example.com");

    let search = option_data
        .get_child("domain_search")
        .and_then(|e| e.get_text())
        .expect("Should have domain_search");
    assert_eq!(search, "example2.com example3.com");

    let ntp = option_data
        .get_child("ntp_servers")
        .and_then(|e| e.get_text())
        .expect("Should have ntp_servers");
    assert_eq!(ntp, "10.22.1.10");

    let autocollect = subnet4
        .get_child("option_data_autocollect")
        .and_then(|e| e.get_text())
        .expect("Should have option_data_autocollect");
    assert_eq!(autocollect, "0");
}

#[test]
fn test_create_options_kea_v6() {
    let input = Cursor::new(TEST_CREATE_OPTIONS_KEA_V6);
    let mut output = Vec::new();
    let options = MigrationOptions {
        create_options: true,
        ..Default::default()
    };

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let kea = root.get_child("Kea").expect("Should have Kea node");
    let dhcp6 = kea.get_child("dhcp6").expect("Should have dhcp6 node");
    let subnet6 = dhcp6
        .get_child("subnets")
        .and_then(|s| s.get_child("subnet6"))
        .expect("Should have subnet6");
    let option_data = subnet6
        .get_child("option_data")
        .expect("Should have option_data");

    let dns = option_data
        .get_child("dns_servers")
        .and_then(|e| e.get_text())
        .expect("Should have dns_servers");
    assert_eq!(dns, "fd00:1234:5678:1::1,fd00:1234:5678:1::2");

    let search = option_data
        .get_child("domain_search")
        .and_then(|e| e.get_text())
        .expect("Should have domain_search");
    assert_eq!(search, "example.com");
}

#[test]
fn test_create_options_kea_existing_skip_and_force() {
    let input = Cursor::new(TEST_CREATE_OPTIONS_KEA_V4_EXISTING);
    let mut output = Vec::new();
    let options = MigrationOptions {
        create_options: true,
        ..Default::default()
    };

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let kea = root.get_child("Kea").expect("Should have Kea node");
    let dhcp4 = kea.get_child("dhcp4").expect("Should have dhcp4 node");
    let subnet4 = dhcp4
        .get_child("subnets")
        .and_then(|s| s.get_child("subnet4"))
        .expect("Should have subnet4");
    let option_data = subnet4
        .get_child("option_data")
        .expect("Should have option_data");
    let dns = option_data
        .get_child("domain_name_servers")
        .and_then(|e| e.get_text())
        .expect("Should have domain_name_servers");
    assert_eq!(dns, "9.9.9.9");

    let mut output_force = Vec::new();
    let options_force = MigrationOptions {
        create_options: true,
        force_options: true,
        ..Default::default()
    };

    convert_config(
        Cursor::new(TEST_CREATE_OPTIONS_KEA_V4_EXISTING),
        &mut output_force,
        &options_force,
    )
    .expect("convert should succeed with force");
    let output_str = String::from_utf8(output_force).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let kea = root.get_child("Kea").expect("Should have Kea node");
    let dhcp4 = kea.get_child("dhcp4").expect("Should have dhcp4 node");
    let subnet4 = dhcp4
        .get_child("subnets")
        .and_then(|s| s.get_child("subnet4"))
        .expect("Should have subnet4");
    let option_data = subnet4
        .get_child("option_data")
        .expect("Should have option_data");
    let dns = option_data
        .get_child("domain_name_servers")
        .and_then(|e| e.get_text())
        .expect("Should have domain_name_servers");
    assert_eq!(dns, "8.8.8.8");
}

#[test]
fn test_create_options_dnsmasq() {
    let input = Cursor::new(TEST_CREATE_OPTIONS_DNSMASQ);
    let mut output = Vec::new();
    let mut options = dnsmasq_options();
    options.create_options = true;

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");

    let dns = dnsmasq_option_value(&root, "opt1", "6", "").expect("dns option should exist");
    assert_eq!(dns, "8.8.8.8,1.1.1.1");

    let routers = dnsmasq_option_value(&root, "opt1", "3", "").expect("router option should exist");
    assert_eq!(routers, "10.22.1.1");

    let domain = dnsmasq_option_value(&root, "opt1", "15", "").expect("domain option should exist");
    assert_eq!(domain, "example.com");

    let search =
        dnsmasq_option_value(&root, "opt1", "119", "").expect("search option should exist");
    assert_eq!(search, "example2.com,example3.com");

    let ntp = dnsmasq_option_value(&root, "opt1", "42", "").expect("ntp option should exist");
    assert_eq!(ntp, "10.22.1.10");

    let v6_dns = dnsmasq_option_value(&root, "lan", "", "23").expect("v6 dns option should exist");
    assert_eq!(v6_dns, "fd00:1234:5678:1::1,fd00:1234:5678:1::2");

    let v6_search =
        dnsmasq_option_value(&root, "lan", "", "24").expect("v6 search option should exist");
    assert_eq!(v6_search, "example.com");
}

#[test]
fn test_create_options_dnsmasq_existing_skip_and_force() {
    let input = Cursor::new(TEST_CREATE_OPTIONS_DNSMASQ_EXISTING);
    let mut output = Vec::new();
    let mut options = dnsmasq_options();
    options.create_options = true;

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let dns = dnsmasq_option_value(&root, "opt1", "6", "").expect("dns option should exist");
    assert_eq!(dns, "9.9.9.9");

    let routers = dnsmasq_option_value(&root, "opt1", "3", "").expect("router option should exist");
    assert_eq!(routers, "10.22.1.1");

    let mut output_force = Vec::new();
    let mut options_force = dnsmasq_options();
    options_force.create_options = true;
    options_force.force_options = true;

    convert_config(
        Cursor::new(TEST_CREATE_OPTIONS_DNSMASQ_EXISTING),
        &mut output_force,
        &options_force,
    )
    .expect("convert should succeed with force");
    let output_str = String::from_utf8(output_force).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let dns = dnsmasq_option_value(&root, "opt1", "6", "").expect("dns option should exist");
    assert_eq!(dns, "8.8.8.8,1.1.1.1");
}
