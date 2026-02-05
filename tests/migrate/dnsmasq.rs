use super::common::*;
use isc2kea::{convert_config, scan_config};
use std::fs;
use std::io::Cursor;
use xmltree::Element;
// dnsmasq backend tests
// ---------------------------------------------------------------------------

#[test]
fn test_dnsmasq_scan_finds_mappings() {
    let input = Cursor::new(TEST_DNSMASQ_XML);
    let options = dnsmasq_options();
    let stats = scan_config(input, &options).expect("scan should succeed");

    assert_eq!(stats.isc_mappings_found, 1, "Should find 1 ISC mapping");
    assert_eq!(
        stats.reservations_to_create, 1,
        "Should plan to create 1 host"
    );
    assert_eq!(stats.reservations_skipped, 0, "Should skip 0 hosts");
    // dnsmasq doesn't use subnets
    assert_eq!(stats.target_subnets_found, 0);
}

#[test]
fn test_dnsmasq_convert_creates_host() {
    let input = Cursor::new(TEST_DNSMASQ_XML);
    let mut output = Vec::new();
    let options = dnsmasq_options();

    let stats = convert_config(input, &mut output, &options).expect("convert should succeed");
    assert_eq!(stats.reservations_to_create, 1, "Should create 1 host");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root = Element::parse(Cursor::new(output_str.as_bytes())).expect("valid XML");

    let hosts = dnsmasq_hosts(&root);

    assert_eq!(hosts.len(), 1, "Should have 1 host entry");

    let host = hosts[0];
    assert!(
        host.attributes.contains_key("uuid"),
        "Host should have UUID"
    );

    let hwaddr = host
        .get_child("hwaddr")
        .and_then(|e| e.get_text())
        .expect("Should have hwaddr");
    assert_eq!(hwaddr, "00:11:22:33:44:55");

    let ip = host
        .get_child("ip")
        .and_then(|e| e.get_text())
        .expect("Should have ip");
    assert_eq!(ip, "192.168.1.10");

    let hostname = host
        .get_child("host")
        .and_then(|e| e.get_text())
        .expect("Should have host");
    assert_eq!(hostname, "testhost");

    let descr = host
        .get_child("descr")
        .and_then(|e| e.get_text())
        .expect("Should have descr");
    assert_eq!(descr, "Test Server");

    // Verify defaults
    let local = host
        .get_child("local")
        .and_then(|e| e.get_text())
        .expect("Should have local");
    assert_eq!(local, "0");

    let ignore = host
        .get_child("ignore")
        .and_then(|e| e.get_text())
        .expect("Should have ignore");
    assert_eq!(ignore, "0");
}

#[test]
fn test_dnsmasq_scan_finds_v6_mappings() {
    let input = Cursor::new(TEST_DNSMASQ_XML_V6);
    let options = dnsmasq_options();
    let stats = scan_config(input, &options).expect("scan should succeed");

    assert_eq!(stats.isc_mappings_found, 0);
    assert_eq!(stats.isc_mappings_v6_found, 1);
    assert_eq!(stats.reservations_v6_to_create, 1);
    assert_eq!(stats.reservations_v6_skipped, 0);
}

#[test]
fn test_dnsmasq_convert_creates_v6_host() {
    let input = Cursor::new(TEST_DNSMASQ_XML_V6);
    let mut output = Vec::new();
    let options = dnsmasq_options();

    let stats = convert_config(input, &mut output, &options).expect("convert should succeed");
    assert_eq!(stats.reservations_v6_to_create, 1);

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root = Element::parse(Cursor::new(output_str.as_bytes())).expect("valid XML");

    let hosts = dnsmasq_hosts(&root);

    assert_eq!(hosts.len(), 1, "Should have 1 host entry");

    let host = hosts[0];

    let ip = host
        .get_child("ip")
        .and_then(|e| e.get_text())
        .expect("Should have ip");
    assert_eq!(ip, "fd00:1234:5678:1::101");

    let client_id = host
        .get_child("client_id")
        .and_then(|e| e.get_text())
        .expect("Should have client_id");
    assert_eq!(client_id, "00:01:00:01:aa:bb:cc:dd:00:11:22:33:44:55");

    let hostname = host
        .get_child("host")
        .and_then(|e| e.get_text())
        .expect("Should have host");
    assert_eq!(hostname, "ipv6examplehost");

    let descr = host
        .get_child("descr")
        .and_then(|e| e.get_text())
        .expect("Should have descr");
    assert_eq!(descr, "test ipv6 static mapping");

    let domain = host
        .get_child("domain")
        .and_then(|e| e.get_text())
        .expect("Should have domain");
    assert_eq!(domain, "example.com");
}

#[test]
fn test_dnsmasq_skip_duplicate_ip() {
    let input = Cursor::new(TEST_DNSMASQ_XML_WITH_EXISTING_IP);
    let options = dnsmasq_options();
    let stats = scan_config(input, &options).expect("scan should succeed");

    assert_eq!(stats.isc_mappings_found, 1);
    assert_eq!(stats.reservations_to_create, 0);
    assert_eq!(stats.reservations_skipped, 1);
}

#[test]
fn test_dnsmasq_skip_duplicate_mac() {
    let input = Cursor::new(TEST_DNSMASQ_XML_WITH_EXISTING_MAC);
    let options = dnsmasq_options();
    let stats = scan_config(input, &options).expect("scan should succeed");

    assert_eq!(stats.isc_mappings_found, 1);
    assert_eq!(stats.reservations_to_create, 0);
    assert_eq!(stats.reservations_skipped, 1);
}

#[test]
fn test_dnsmasq_skip_duplicate_v6_ip() {
    let input = Cursor::new(TEST_DNSMASQ_XML_V6_WITH_EXISTING_IP);
    let options = dnsmasq_options();
    let stats = scan_config(input, &options).expect("scan should succeed");

    assert_eq!(stats.isc_mappings_v6_found, 1);
    assert_eq!(stats.reservations_v6_to_create, 0);
    assert_eq!(stats.reservations_v6_skipped, 1);
}

#[test]
fn test_dnsmasq_skip_duplicate_v6_duid() {
    let input = Cursor::new(TEST_DNSMASQ_XML_V6_WITH_EXISTING_DUID);
    let options = dnsmasq_options();
    let stats = scan_config(input, &options).expect("scan should succeed");

    assert_eq!(stats.isc_mappings_v6_found, 1);
    assert_eq!(stats.reservations_v6_to_create, 0);
    assert_eq!(stats.reservations_v6_skipped, 1);
}

#[test]
fn test_dnsmasq_error_when_not_configured() {
    let xml_no_dnsmasq = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <lan>
            <ipaddr>192.168.1.1</ipaddr>
            <subnet>24</subnet>
        </lan>
    </interfaces>
    <dhcpd>
        <lan>
            <staticmap>
                <mac>00:11:22:33:44:55</mac>
                <ipaddr>192.168.1.10</ipaddr>
                <hostname>testhost</hostname>
            </staticmap>
        </lan>
    </dhcpd>
</opnsense>
"#;

    let input = Cursor::new(xml_no_dnsmasq);
    let options = dnsmasq_options();
    let result = scan_config(input, &options);

    assert!(
        result.is_err(),
        "Should fail when dnsmasq is not configured"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("dnsmasq DHCPv4 not configured"),
        "Error should mention dnsmasq not configured, got: {}",
        err_msg
    );
}

#[test]
fn test_dnsmasq_fail_if_existing_v6_client_id() {
    let input = Cursor::new(TEST_DNSMASQ_XML_V6_WITH_EXISTING_CLIENT_ID_ONLY);
    let mut options = dnsmasq_options();
    options.fail_if_existing = true;
    let result = scan_config(input, &options);

    assert!(result.is_err(), "Should fail with existing dnsmasq hosts");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Existing dnsmasq hosts found"),
        "Error should mention existing dnsmasq hosts, got: {}",
        err_msg
    );
}

#[test]
fn test_dnsmasq_convert_matches_golden_fixtures() {
    let input = fs::read_to_string("fixtures/dnsmasq_minimal.xml")
        .expect("dnsmasq input fixture should be readable");
    let expected = fs::read_to_string("fixtures/golden_expected_dnsmasq.xml")
        .expect("dnsmasq expected fixture should be readable");

    let mut output = Vec::new();
    let options = dnsmasq_options();

    convert_config(Cursor::new(input.as_bytes()), &mut output, &options)
        .expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let output_root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let expected_root =
        Element::parse(Cursor::new(expected.as_bytes())).expect("expected should be valid XML");

    let output_dnsmasq =
        find_descendant_ci(&output_root, "dnsmasq").expect("output should have dnsmasq");
    let expected_dnsmasq =
        find_descendant_ci(&expected_root, "dnsmasq").expect("expected should have dnsmasq");

    assert_eq!(
        dnsmasq_hosts_as_fields(output_dnsmasq),
        dnsmasq_hosts_as_fields(expected_dnsmasq)
    );
}

#[test]
fn test_dnsmasq_convert_matches_golden_fixtures_v6() {
    let input = fs::read_to_string("fixtures/dnsmasq_v6_minimal.xml")
        .expect("dnsmasq v6 input fixture should be readable");
    let expected = fs::read_to_string("fixtures/golden_expected_dnsmasq_v6.xml")
        .expect("dnsmasq v6 expected fixture should be readable");

    let mut output = Vec::new();
    let options = dnsmasq_options();

    convert_config(Cursor::new(input.as_bytes()), &mut output, &options)
        .expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let output_root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");
    let expected_root =
        Element::parse(Cursor::new(expected.as_bytes())).expect("expected should be valid XML");

    let output_dnsmasq =
        find_descendant_ci(&output_root, "dnsmasq").expect("output should have dnsmasq");
    let expected_dnsmasq =
        find_descendant_ci(&expected_root, "dnsmasq").expect("expected should have dnsmasq");

    assert_eq!(
        dnsmasq_hosts_as_fields(output_dnsmasq),
        dnsmasq_hosts_as_fields(expected_dnsmasq)
    );
}

#[test]
fn test_enable_backend_dnsmasq_enables_service() {
    let input = Cursor::new(TEST_ENABLE_BACKEND_DNSMASQ);
    let mut output = Vec::new();
    let mut options = dnsmasq_options();
    options.create_subnets = true;
    options.enable_backend = true;

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");

    // Check dnsmasq is enabled
    let dnsmasq = find_descendant_ci(&root, "dnsmasq").expect("Should have dnsmasq node");
    let enable = dnsmasq
        .get_child("enable")
        .expect("Should have enable element");
    let enable_value = enable.get_text().expect("Should have enable value");
    assert_eq!(enable_value, "1", "dnsmasq should be enabled");
}

#[test]
fn test_enable_backend_dnsmasq_disables_isc() {
    let input = Cursor::new(TEST_ENABLE_BACKEND_DNSMASQ);
    let mut output = Vec::new();
    let mut options = dnsmasq_options();
    options.create_subnets = true;
    options.enable_backend = true;

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");

    // Check ISC DHCP is disabled on opt1
    let dhcpd = root.get_child("dhcpd").expect("Should have dhcpd node");
    let opt1 = dhcpd.get_child("opt1").expect("Should have opt1 node");
    let enable = opt1
        .get_child("enable")
        .expect("Should have enable element");
    let enable_value = enable.get_text().unwrap_or_default();
    assert!(
        enable_value.is_empty(),
        "ISC DHCP should be disabled (empty enable)"
    );
}
