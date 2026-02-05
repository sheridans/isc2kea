use super::common::*;
use isc2kea::{convert_config, scan_config, MigrationOptions};
use std::fs;
use std::io::Cursor;
use xmltree::Element;

#[test]
fn test_scan_finds_mappings() {
    let input = Cursor::new(TEST_XML);
    let options = MigrationOptions::default();
    let stats = scan_config(input, &options).expect("scan should succeed");

    assert_eq!(stats.isc_mappings_found, 1, "Should find 1 ISC mapping");
    assert_eq!(stats.target_subnets_found, 1, "Should find 1 Kea subnet");
    assert_eq!(
        stats.reservations_to_create, 1,
        "Should plan to create 1 reservation"
    );
    assert_eq!(stats.reservations_skipped, 0, "Should skip 0 reservations");
}

#[test]
fn test_scan_finds_v6_mappings() {
    let input = Cursor::new(TEST_XML_V6);
    let options = MigrationOptions::default();
    let stats = scan_config(input, &options).expect("scan should succeed");

    assert_eq!(stats.isc_mappings_found, 0);
    assert_eq!(stats.isc_mappings_v6_found, 1);
    assert_eq!(stats.target_subnets_found, 0);
    assert_eq!(stats.target_subnets_v6_found, 1);
    assert_eq!(stats.reservations_v6_to_create, 1);
    assert_eq!(stats.reservations_v6_skipped, 0);
}

#[test]
fn test_convert_creates_reservation() {
    let input = Cursor::new(TEST_XML);
    let mut output = Vec::new();
    let options = MigrationOptions::default();

    let stats = convert_config(input, &mut output, &options).expect("convert should succeed");

    assert_eq!(
        stats.reservations_to_create, 1,
        "Should create 1 reservation"
    );

    // Parse the output and verify the reservation was added
    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let output_cursor = Cursor::new(output_str.as_bytes());
    let root = xmltree::Element::parse(output_cursor).expect("output should be valid XML");

    // Navigate to reservations
    let kea = root.get_child("Kea").expect("Should have Kea node");
    let dhcp4 = kea.get_child("dhcp4").expect("Should have dhcp4 node");
    let reservations = dhcp4
        .get_child("reservations")
        .expect("Should have reservations node");

    // Count reservation elements
    let reservation_count = reservations
        .children
        .iter()
        .filter(|child| {
            child
                .as_element()
                .map(|e| e.name == "reservation")
                .unwrap_or(false)
        })
        .count();

    assert_eq!(reservation_count, 1, "Should have 1 reservation in output");

    // Verify reservation content
    let reservation = reservations
        .children
        .iter()
        .find_map(|child| child.as_element())
        .and_then(|e| {
            if e.name == "reservation" {
                Some(e)
            } else {
                None
            }
        })
        .expect("Should have a reservation element");

    assert!(
        reservation.attributes.contains_key("uuid"),
        "Reservation should have UUID"
    );

    let ip = reservation
        .get_child("ip_address")
        .and_then(|e| e.get_text())
        .expect("Should have ip_address");
    assert_eq!(ip, "192.168.1.10");

    let hw = reservation
        .get_child("hw_address")
        .and_then(|e| e.get_text())
        .expect("Should have hw_address");
    assert_eq!(hw, "00:11:22:33:44:55");

    let subnet = reservation
        .get_child("subnet")
        .and_then(|e| e.get_text())
        .expect("Should have subnet");
    assert_eq!(subnet, "test-subnet-uuid-1234");

    let hostname = reservation
        .get_child("hostname")
        .and_then(|e| e.get_text())
        .expect("Should have hostname");
    assert_eq!(hostname, "testhost");

    let description = reservation
        .get_child("description")
        .and_then(|e| e.get_text())
        .expect("Should have description");
    assert_eq!(description, "Test Server");
}

#[test]
fn test_convert_creates_v6_reservation() {
    let input = Cursor::new(TEST_XML_V6);
    let mut output = Vec::new();
    let options = MigrationOptions::default();

    let stats = convert_config(input, &mut output, &options).expect("convert should succeed");

    assert_eq!(stats.reservations_v6_to_create, 1);

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let output_cursor = Cursor::new(output_str.as_bytes());
    let root = xmltree::Element::parse(output_cursor).expect("output should be valid XML");

    let kea = root.get_child("Kea").expect("Should have Kea node");
    let dhcp6 = kea.get_child("dhcp6").expect("Should have dhcp6 node");
    let reservations = dhcp6
        .get_child("reservations")
        .expect("Should have reservations node");

    let reservation = reservations
        .children
        .iter()
        .find_map(|child| child.as_element())
        .and_then(|e| {
            if e.name == "reservation" {
                Some(e)
            } else {
                None
            }
        })
        .expect("Should have a reservation element");

    let ip = reservation
        .get_child("ip_address")
        .and_then(|e| e.get_text())
        .expect("Should have ip_address");
    assert_eq!(ip, "2001:db8:42::10");

    let duid = reservation
        .get_child("duid")
        .and_then(|e| e.get_text())
        .expect("Should have duid");
    assert_eq!(duid, "00:01:00:01:aa:bb:cc:dd:00:11:22:33:44:55");

    let hostname = reservation
        .get_child("hostname")
        .and_then(|e| e.get_text())
        .expect("Should have hostname");
    assert_eq!(hostname, "host1");

    let domain = reservation
        .get_child("domain_search")
        .and_then(|e| e.get_text())
        .expect("Should have domain_search");
    assert_eq!(domain, "mydomain.local");

    let description = reservation
        .get_child("description")
        .and_then(|e| e.get_text())
        .expect("Should have description");
    assert_eq!(description, "test device 1");
}
#[test]
fn test_skip_duplicate_ip() {
    let xml_with_existing = r#"<?xml version="1.0"?>
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
    <Kea>
        <dhcp4>
            <subnets>
                <subnet4 uuid="test-subnet-uuid-1234">
                    <subnet>192.168.1.0/24</subnet>
                </subnet4>
            </subnets>
            <reservations>
                <reservation uuid="existing-reservation">
                    <ip_address>192.168.1.10</ip_address>
                    <hw_address>99:99:99:99:99:99</hw_address>
                </reservation>
            </reservations>
        </dhcp4>
    </Kea>
</opnsense>
"#;

    let input = Cursor::new(xml_with_existing);
    let options = MigrationOptions::default();
    let stats = scan_config(input, &options).expect("scan should succeed");

    assert_eq!(stats.isc_mappings_found, 1, "Should find 1 ISC mapping");
    assert_eq!(
        stats.reservations_to_create, 0,
        "Should not create any reservations"
    );
    assert_eq!(
        stats.reservations_skipped, 1,
        "Should skip 1 duplicate reservation"
    );
}

#[test]
fn test_skip_duplicate_v6_duid() {
    let input = Cursor::new(TEST_XML_V6_WITH_EXISTING_DUID);
    let options = MigrationOptions::default();
    let stats = scan_config(input, &options).expect("scan should succeed");

    assert_eq!(stats.isc_mappings_v6_found, 1);
    assert_eq!(stats.reservations_v6_to_create, 0);
    assert_eq!(stats.reservations_v6_skipped, 1);
}
#[test]
fn test_error_on_no_matching_subnet() {
    let xml_no_subnet = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <lan>
            <ipaddr>172.16.0.1</ipaddr>
            <subnet>24</subnet>
        </lan>
    </interfaces>
    <dhcpd>
        <lan>
            <staticmap>
                <mac>00:11:22:33:44:55</mac>
                <ipaddr>172.16.0.10</ipaddr>
                <hostname>testhost</hostname>
            </staticmap>
        </lan>
    </dhcpd>
    <Kea>
        <dhcp4>
            <subnets>
                <subnet4 uuid="test-subnet-uuid-1234">
                    <subnet>192.168.1.0/24</subnet>
                </subnet4>
            </subnets>
        </dhcp4>
    </Kea>
</opnsense>
"#;

    let input = Cursor::new(xml_no_subnet);
    let options = MigrationOptions::default();
    let result = scan_config(input, &options);

    assert!(
        result.is_err(),
        "Should fail when IP doesn't match any subnet"
    );
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("does not match any configured subnet"));
}

#[test]
fn test_error_on_interface_mismatch() {
    let xml_iface_mismatch = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <lan>
            <ipaddr>10.0.0.1</ipaddr>
            <subnet>24</subnet>
        </lan>
        <opt1>
            <ipaddr>192.168.1.1</ipaddr>
            <subnet>24</subnet>
        </opt1>
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
    <Kea>
        <dhcp4>
            <subnets>
                <subnet4 uuid="test-subnet-uuid-1234">
                    <subnet>192.168.1.0/24</subnet>
                </subnet4>
            </subnets>
        </dhcp4>
    </Kea>
</opnsense>
"#;

    let input = Cursor::new(xml_iface_mismatch);
    let options = MigrationOptions::default();
    let result = scan_config(input, &options);

    assert!(
        result.is_err(),
        "Should fail when ISC interface mismatches IP"
    );
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("maps to interface"));
}

#[test]
fn test_dnsmasq_error_on_interface_mismatch() {
    let xml_iface_mismatch = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <lan>
            <ipaddr>10.0.0.1</ipaddr>
            <subnet>24</subnet>
        </lan>
        <opt1>
            <ipaddr>192.168.1.1</ipaddr>
            <subnet>24</subnet>
        </opt1>
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
    <dnsmasq></dnsmasq>
</opnsense>
"#;

    let input = Cursor::new(xml_iface_mismatch);
    let options = dnsmasq_options();
    let result = scan_config(input, &options);

    assert!(
        result.is_err(),
        "Should fail when ISC interface mismatches IP"
    );
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("maps to interface"));
}

#[test]
fn test_error_when_kea_not_configured() {
    let xml_no_kea = r#"<?xml version="1.0"?>
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

    let input = Cursor::new(xml_no_kea);
    let options = MigrationOptions::default();
    let result = scan_config(input, &options);

    assert!(result.is_err(), "Should fail when Kea is not configured");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Kea DHCPv4 not configured"),
        "Error should say 'Kea DHCPv4 not configured', got: {}",
        err_msg
    );
}

#[test]
fn test_error_when_kea_has_no_subnets() {
    let xml_kea_no_subnets = r#"<?xml version="1.0"?>
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
    <Kea>
        <dhcp4>
            <subnets>
            </subnets>
        </dhcp4>
    </Kea>
</opnsense>
"#;

    let input = Cursor::new(xml_kea_no_subnets);
    let options = MigrationOptions::default();
    let result = scan_config(input, &options);

    assert!(result.is_err(), "Should fail when Kea has no subnets");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("No Kea subnets found"),
        "Error should say 'No Kea subnets found', got: {}",
        err_msg
    );
}

#[test]
fn test_handles_isc_duplicates() {
    let xml_with_isc_duplicates = r#"<?xml version="1.0"?>
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
                <hostname>first</hostname>
            </staticmap>
            <staticmap>
                <mac>aa:bb:cc:dd:ee:ff</mac>
                <ipaddr>192.168.1.10</ipaddr>
                <hostname>duplicate</hostname>
            </staticmap>
        </lan>
    </dhcpd>
    <Kea>
        <dhcp4>
            <subnets>
                <subnet4 uuid="test-subnet-uuid-1234">
                    <subnet>192.168.1.0/24</subnet>
                </subnet4>
            </subnets>
        </dhcp4>
    </Kea>
</opnsense>
"#;

    let input = Cursor::new(xml_with_isc_duplicates);
    let options = MigrationOptions::default();
    let stats = scan_config(input, &options).expect("scan should succeed");

    assert_eq!(stats.isc_mappings_found, 2, "Should find 2 ISC mappings");
    assert_eq!(
        stats.reservations_to_create, 1,
        "Should only create 1 reservation"
    );
    assert_eq!(stats.reservations_skipped, 1, "Should skip 1 duplicate");
}

#[test]
fn test_case_insensitive_kea_tags() {
    let xml_lowercase_kea = r#"<?xml version="1.0"?>
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
    <kea>
        <dhcp4>
            <subnets>
                <subnet4 uuid="test-subnet-uuid-1234">
                    <subnet>192.168.1.0/24</subnet>
                </subnet4>
            </subnets>
        </dhcp4>
    </kea>
</opnsense>
"#;

    let input = Cursor::new(xml_lowercase_kea);
    let options = MigrationOptions::default();
    let stats = scan_config(input, &options).expect("scan should succeed with lowercase <kea>");

    assert_eq!(
        stats.target_subnets_found, 1,
        "Should find subnet with lowercase <kea>"
    );
    assert_eq!(
        stats.reservations_to_create, 1,
        "Should plan to create reservation"
    );
}

#[test]
fn test_case_insensitive_isc_tags() {
    let xml_uppercase_dhcpd = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <lan>
            <ipaddr>192.168.1.1</ipaddr>
            <subnet>24</subnet>
        </lan>
    </interfaces>
    <DHCPD>
        <lan>
            <STATICMAP>
                <MAC>00:11:22:33:44:55</MAC>
                <IPADDR>192.168.1.10</IPADDR>
                <HOSTNAME>testhost</HOSTNAME>
            </STATICMAP>
        </lan>
    </DHCPD>
    <Kea>
        <dhcp4>
            <subnets>
                <subnet4 uuid="test-subnet-uuid-1234">
                    <subnet>192.168.1.0/24</subnet>
                </subnet4>
            </subnets>
        </dhcp4>
    </Kea>
</opnsense>
"#;

    let input = Cursor::new(xml_uppercase_dhcpd);
    let options = MigrationOptions::default();
    let stats = scan_config(input, &options).expect("scan should succeed with uppercase ISC tags");

    assert_eq!(
        stats.isc_mappings_found, 1,
        "Should find ISC mapping with uppercase tags"
    );
    assert_eq!(
        stats.reservations_to_create, 1,
        "Should plan to create reservation"
    );
}

#[test]
fn test_fallback_kea_schema() {
    let xml_subnet4_direct = r#"<?xml version="1.0"?>
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
    <Kea>
        <dhcp4>
            <subnet4 uuid="test-subnet-uuid-1234">
                <subnet>192.168.1.0/24</subnet>
            </subnet4>
        </dhcp4>
    </Kea>
</opnsense>
"#;

    let input = Cursor::new(xml_subnet4_direct);
    let options = MigrationOptions::default();
    let stats = scan_config(input, &options)
        .expect("scan should succeed with subnet4 directly under dhcp4");

    assert_eq!(
        stats.target_subnets_found, 1,
        "Should find subnet4 directly under dhcp4"
    );
    assert_eq!(
        stats.reservations_to_create, 1,
        "Should plan to create reservation"
    );
}

#[test]
fn test_convert_matches_golden_fixtures() {
    let input = fs::read_to_string("fixtures/golden_input.xml")
        .expect("golden input fixture should be readable");
    let expected = fs::read_to_string("fixtures/golden_expected_kea.xml")
        .expect("golden expected fixture should be readable");

    let mut output = Vec::new();
    let options = MigrationOptions::default();

    convert_config(Cursor::new(input.as_bytes()), &mut output, &options)
        .expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let output_cursor = Cursor::new(output_str.as_bytes());
    let output_root = Element::parse(output_cursor).expect("output should be valid XML");

    let expected_cursor = Cursor::new(expected.as_bytes());
    let expected_root = Element::parse(expected_cursor).expect("expected should be valid XML");

    let output_kea = find_descendant_ci(&output_root, "Kea").expect("output should have Kea node");
    let expected_kea =
        find_descendant_ci(&expected_root, "Kea").expect("expected should have Kea node");

    let output_dhcp4 = output_kea
        .get_child("dhcp4")
        .expect("output should have dhcp4");
    let expected_dhcp4 = expected_kea
        .get_child("dhcp4")
        .expect("expected should have dhcp4");
    assert_eq!(
        reservations_as_fields(output_dhcp4),
        reservations_as_fields(expected_dhcp4)
    );

    let output_dhcp6 = output_kea
        .get_child("dhcp6")
        .expect("output should have dhcp6");
    let expected_dhcp6 = expected_kea
        .get_child("dhcp6")
        .expect("expected should have dhcp6");
    assert_eq!(
        reservations_as_fields(output_dhcp6),
        reservations_as_fields(expected_dhcp6)
    );
}

#[test]
fn test_enable_backend_kea_enables_dhcp4() {
    let input = Cursor::new(TEST_ENABLE_BACKEND_KEA);
    let mut output = Vec::new();
    let options = MigrationOptions {
        create_subnets: true,
        enable_backend: true,
        ..Default::default()
    };

    convert_config(input, &mut output, &options).expect("convert should succeed");

    let output_str = String::from_utf8(output).expect("output should be valid UTF-8");
    let root =
        Element::parse(Cursor::new(output_str.as_bytes())).expect("output should be valid XML");

    // Check Kea dhcp4 is enabled
    let kea = root.get_child("Kea").expect("Should have Kea node");
    let dhcp4 = kea.get_child("dhcp4").expect("Should have dhcp4 node");
    let general = dhcp4
        .get_child("general")
        .expect("Should have general node");
    let enabled = general
        .get_child("enabled")
        .expect("Should have enabled element");
    let enabled_value = enabled.get_text().expect("Should have enabled value");
    assert_eq!(enabled_value, "1", "Kea dhcp4 should be enabled");
}

#[test]
fn test_enable_backend_kea_disables_isc() {
    let input = Cursor::new(TEST_ENABLE_BACKEND_KEA);
    let mut output = Vec::new();
    let options = MigrationOptions {
        create_subnets: true,
        enable_backend: true,
        ..Default::default()
    };

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

#[test]
fn test_enable_backend_kea_disables_isc_without_ranges() {
    let xml_no_ranges = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <opt1>
            <ipaddr>10.22.1.1</ipaddr>
            <subnet>24</subnet>
        </opt1>
    </interfaces>
    <dhcpd>
        <opt1>
            <staticmap>
                <mac>04:d9:f5:cb:9b:54</mac>
                <ipaddr>10.22.1.50</ipaddr>
            </staticmap>
        </opt1>
    </dhcpd>
    <Kea>
        <dhcp4>
            <subnets>
                <subnet4 uuid="test-subnet-uuid-1234">
                    <subnet>10.22.1.0/24</subnet>
                </subnet4>
            </subnets>
        </dhcp4>
    </Kea>
</opnsense>
"#;

    let input = Cursor::new(xml_no_ranges);
    let mut output = Vec::new();
    let options = MigrationOptions {
        enable_backend: true,
        ..Default::default()
    };

    let stats = convert_config(input, &mut output, &options).expect("convert should succeed");
    assert_eq!(stats.isc_disabled_v4, vec!["opt1"]);
}

#[test]
fn test_enable_backend_kea_sets_enabled_tag() {
    let xml_missing_enabled = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <opt1>
            <ipaddr>10.22.1.1</ipaddr>
            <subnet>24</subnet>
        </opt1>
    </interfaces>
    <dhcpd>
        <opt1>
            <range>
                <from>10.22.1.100</from>
                <to>10.22.1.200</to>
            </range>
            <staticmap>
                <mac>04:d9:f5:cb:9b:54</mac>
                <ipaddr>10.22.1.50</ipaddr>
            </staticmap>
        </opt1>
    </dhcpd>
    <Kea>
        <dhcp4>
            <general></general>
            <subnets>
                <subnet4 uuid="test-subnet-uuid-1234">
                    <subnet>10.22.1.0/24</subnet>
                </subnet4>
            </subnets>
            <reservations></reservations>
        </dhcp4>
    </Kea>
</opnsense>
"#;

    let input = Cursor::new(xml_missing_enabled);
    let mut output = Vec::new();
    let options = MigrationOptions {
        enable_backend: true,
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
    let enabled = general
        .get_child("enabled")
        .expect("Should have enabled element");
    let enabled_value = enabled.get_text().expect("Should have enabled value");
    assert_eq!(enabled_value, "1", "Kea dhcp4 should be enabled");
}

#[test]
fn test_enable_backend_kea_stats() {
    let input = Cursor::new(TEST_ENABLE_BACKEND_KEA);
    let mut output = Vec::new();
    let options = MigrationOptions {
        create_subnets: true,
        enable_backend: true,
        ..Default::default()
    };

    let stats = convert_config(input, &mut output, &options).expect("convert should succeed");

    assert_eq!(stats.interfaces_configured, vec!["opt1"]);
    assert_eq!(stats.isc_disabled_v4, vec!["opt1"]);
    assert!(stats.isc_disabled_v6.is_empty());
    assert!(stats.backend_enabled_v4);
    assert!(!stats.backend_enabled_v6);
}

// ---------------------------------------------------------------------------
