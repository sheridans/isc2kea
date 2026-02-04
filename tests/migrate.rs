use isc2kea::{convert_config, scan_config, Backend, MigrationOptions};
use std::fs;
use std::io::Cursor;
use xmltree::Element;

fn find_descendant_ci<'a>(elem: &'a Element, name: &str) -> Option<&'a Element> {
    if elem.name.eq_ignore_ascii_case(name) {
        return Some(elem);
    }

    for child in &elem.children {
        if let Some(child_elem) = child.as_element() {
            if let Some(found) = find_descendant_ci(child_elem, name) {
                return Some(found);
            }
        }
    }

    None
}

fn reservation_fields(reservation: &Element) -> Vec<(String, String)> {
    reservation
        .children
        .iter()
        .filter_map(|child| child.as_element())
        .map(|child| {
            let text = child
                .get_text()
                .map(|value| value.to_string())
                .unwrap_or_default();
            (child.name.clone(), text)
        })
        .collect()
}

fn reservations_as_fields(dhcp: &Element) -> Vec<Vec<(String, String)>> {
    let reservations = dhcp
        .get_child("reservations")
        .expect("Should have reservations node");

    reservations
        .children
        .iter()
        .filter_map(|child| child.as_element())
        .filter(|elem| elem.name == "reservation")
        .map(reservation_fields)
        .collect()
}

/// Extract child element fields from a dnsmasq hosts element (same structure as reservation_fields)
fn dnsmasq_host_fields(host: &Element) -> Vec<(String, String)> {
    host.children
        .iter()
        .filter_map(|child| child.as_element())
        .map(|child| {
            let text = child
                .get_text()
                .map(|value| value.to_string())
                .unwrap_or_default();
            (child.name.clone(), text)
        })
        .collect()
}

fn dnsmasq_hosts_as_fields(dnsmasq: &Element) -> Vec<Vec<(String, String)>> {
    dnsmasq
        .children
        .iter()
        .filter_map(|child| child.as_element())
        .filter(|elem| elem.name == "hosts")
        .map(dnsmasq_host_fields)
        .collect()
}

fn dnsmasq_hosts(root: &Element) -> Vec<&Element> {
    let dnsmasq = find_descendant_ci(root, "dnsmasq").expect("Should have dnsmasq node");
    dnsmasq
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .filter(|e| e.name == "hosts")
        .collect()
}

fn dnsmasq_option_value(
    root: &Element,
    iface: &str,
    option: &str,
    option6: &str,
) -> Option<String> {
    let dnsmasq = find_descendant_ci(root, "dnsmasq").expect("Should have dnsmasq node");
    for elem in dnsmasq
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .filter(|e| e.name == "dhcp_options")
    {
        let opt_type = elem
            .get_child("type")
            .and_then(|e| e.get_text())
            .map(|s| s.to_string())
            .unwrap_or_default();
        if opt_type != "set" {
            continue;
        }
        let iface_text = elem
            .get_child("interface")
            .and_then(|e| e.get_text())
            .map(|s| s.to_string())
            .unwrap_or_default();
        let opt = elem
            .get_child("option")
            .and_then(|e| e.get_text())
            .map(|s| s.to_string())
            .unwrap_or_default();
        let opt6 = elem
            .get_child("option6")
            .and_then(|e| e.get_text())
            .map(|s| s.to_string())
            .unwrap_or_default();
        if iface_text == iface && opt == option && opt6 == option6 {
            return elem
                .get_child("value")
                .and_then(|e| e.get_text())
                .map(|s| s.to_string());
        }
    }
    None
}

fn dnsmasq_options() -> MigrationOptions {
    MigrationOptions {
        backend: Backend::Dnsmasq,
        ..Default::default()
    }
}

fn dnsmasq_options_create_subnets() -> MigrationOptions {
    MigrationOptions {
        backend: Backend::Dnsmasq,
        create_subnets: true,
        ..Default::default()
    }
}

const TEST_XML: &str = r#"<?xml version="1.0"?>
<opnsense>
    <dhcpd>
        <lan>
            <staticmap>
                <mac>00:11:22:33:44:55</mac>
                <ipaddr>192.168.1.10</ipaddr>
                <hostname>testhost</hostname>
                <descr>Test Server</descr>
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

const TEST_XML_V6: &str = r#"<?xml version="1.0"?>
<opnsense>
    <dhcpdv6>
        <opt2>
            <staticmap>
                <duid>00:01:00:01:aa:bb:cc:dd:00:11:22:33:44:55</duid>
                <ipaddrv6>2001:db8:42::10</ipaddrv6>
                <hostname>host1</hostname>
                <descr>test device 1</descr>
                <domainsearchlist>mydomain.local</domainsearchlist>
            </staticmap>
        </opt2>
    </dhcpdv6>
    <Kea>
        <dhcp6>
            <subnets>
                <subnet6 uuid="v6-subnet-uuid-1234">
                    <subnet>2001:db8:42::/64</subnet>
                </subnet6>
            </subnets>
        </dhcp6>
    </Kea>
</opnsense>
"#;

const TEST_DNSMASQ_XML: &str = r#"<?xml version="1.0"?>
<opnsense>
    <dhcpd>
        <lan>
            <staticmap>
                <mac>00:11:22:33:44:55</mac>
                <ipaddr>192.168.1.10</ipaddr>
                <hostname>testhost</hostname>
                <descr>Test Server</descr>
            </staticmap>
        </lan>
    </dhcpd>
    <dnsmasq>
    </dnsmasq>
</opnsense>
"#;

const TEST_DNSMASQ_XML_WITH_EXISTING_IP: &str = r#"<?xml version="1.0"?>
<opnsense>
    <dhcpd>
        <lan>
            <staticmap>
                <mac>00:11:22:33:44:55</mac>
                <ipaddr>192.168.1.10</ipaddr>
                <hostname>testhost</hostname>
            </staticmap>
        </lan>
    </dhcpd>
    <dnsmasq>
        <hosts uuid="existing-host-1">
            <hwaddr>99:99:99:99:99:99</hwaddr>
            <ip>192.168.1.10</ip>
            <host>existing</host>
        </hosts>
    </dnsmasq>
</opnsense>
"#;

const TEST_DNSMASQ_XML_WITH_EXISTING_MAC: &str = r#"<?xml version="1.0"?>
<opnsense>
    <dhcpd>
        <lan>
            <staticmap>
                <mac>00:11:22:33:44:55</mac>
                <ipaddr>192.168.1.10</ipaddr>
                <hostname>testhost</hostname>
            </staticmap>
        </lan>
    </dhcpd>
    <dnsmasq>
        <hosts uuid="existing-host-1">
            <hwaddr>00:11:22:33:44:55</hwaddr>
            <ip>192.168.1.99</ip>
            <host>existing</host>
        </hosts>
    </dnsmasq>
</opnsense>
"#;

const TEST_DNSMASQ_XML_V6: &str = r#"<?xml version="1.0"?>
<opnsense>
    <dhcpdv6>
        <lan>
            <staticmap>
                <duid>00:01:00:01:aa:bb:cc:dd:00:11:22:33:44:55</duid>
                <ipaddrv6>fd00:1234:5678:1::101</ipaddrv6>
                <hostname>ipv6examplehost</hostname>
                <descr>test ipv6 static mapping</descr>
                <domainsearchlist>example.com other.example</domainsearchlist>
            </staticmap>
        </lan>
    </dhcpdv6>
    <dnsmasq>
    </dnsmasq>
</opnsense>
"#;

const TEST_DNSMASQ_XML_V6_WITH_EXISTING_IP: &str = r#"<?xml version="1.0"?>
<opnsense>
    <dhcpdv6>
        <lan>
            <staticmap>
                <duid>00:01:00:01:aa:bb:cc:dd:00:11:22:33:44:55</duid>
                <ipaddrv6>fd00:1234:5678:1::101</ipaddrv6>
                <hostname>ipv6examplehost</hostname>
            </staticmap>
        </lan>
    </dhcpdv6>
    <dnsmasq>
        <hosts uuid="existing-host-v6">
            <host>existingv6</host>
            <ip>fd00:1234:5678:1::101</ip>
            <client_id>00:01:00:01:11:22:33:44:55:66:77:88:99:aa</client_id>
            <hwaddr/>
        </hosts>
    </dnsmasq>
</opnsense>
"#;

const TEST_DNSMASQ_XML_V6_WITH_EXISTING_DUID: &str = r#"<?xml version="1.0"?>
<opnsense>
    <dhcpdv6>
        <lan>
            <staticmap>
                <duid>00:01:00:01:aa:bb:cc:dd:00:11:22:33:44:55</duid>
                <ipaddrv6>fd00:1234:5678:1::101</ipaddrv6>
                <hostname>ipv6examplehost</hostname>
            </staticmap>
        </lan>
    </dhcpdv6>
    <dnsmasq>
        <hosts uuid="existing-host-v6">
            <host>existingv6</host>
            <ip>fd00:1234:5678:1::200</ip>
            <client_id>00:01:00:01:aa:bb:cc:dd:00:11:22:33:44:55</client_id>
            <hwaddr/>
        </hosts>
    </dnsmasq>
</opnsense>
"#;

const TEST_DNSMASQ_XML_V6_WITH_EXISTING_CLIENT_ID_ONLY: &str = r#"<?xml version="1.0"?>
<opnsense>
    <dhcpdv6>
        <lan>
            <staticmap>
                <duid>00:01:00:01:aa:bb:cc:dd:00:11:22:33:44:55</duid>
                <ipaddrv6>fd00:1234:5678:1::101</ipaddrv6>
                <hostname>ipv6examplehost</hostname>
            </staticmap>
        </lan>
    </dhcpdv6>
    <dnsmasq>
        <hosts uuid="existing-host-v6">
            <host>existingv6</host>
            <client_id>00:01:00:01:aa:bb:cc:dd:00:11:22:33:44:55</client_id>
        </hosts>
    </dnsmasq>
</opnsense>
"#;
const TEST_XML_V6_WITH_EXISTING_DUID: &str = r#"<?xml version="1.0"?>
<opnsense>
    <dhcpdv6>
        <opt2>
            <staticmap>
                <duid>00:01:00:01:aa:bb:cc:dd:00:11:22:33:44:55</duid>
                <ipaddrv6>2001:db8:42::10</ipaddrv6>
                <hostname>host1</hostname>
            </staticmap>
        </opt2>
    </dhcpdv6>
    <Kea>
        <dhcp6>
            <subnets>
                <subnet6 uuid="v6-subnet-uuid-1234">
                    <subnet>2001:db8:42::/64</subnet>
                </subnet6>
            </subnets>
            <reservations>
                <reservation uuid="existing-v6">
                    <subnet>v6-subnet-uuid-1234</subnet>
                    <ip_address>2001:db8:42::99</ip_address>
                    <duid>00:01:00:01:aa:bb:cc:dd:00:11:22:33:44:55</duid>
                </reservation>
            </reservations>
        </dhcp6>
    </Kea>
</opnsense>
"#;

const TEST_CREATE_SUBNETS_KEA_V4: &str = r#"<?xml version="1.0"?>
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
                <ipaddr>10.22.1.100</ipaddr>
            </staticmap>
        </opt1>
    </dhcpd>
    <Kea>
        <dhcp4>
            <subnets></subnets>
            <reservations></reservations>
        </dhcp4>
    </Kea>
</opnsense>
"#;

const TEST_CREATE_SUBNETS_KEA_V6: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <lan>
            <ipaddrv6>fd00:1234:5678:1::1</ipaddrv6>
            <subnetv6>64</subnetv6>
        </lan>
    </interfaces>
    <dhcpdv6>
        <lan>
            <range>
                <from>fd00:1234:5678:1::100</from>
                <to>fd00:1234:5678:1::200</to>
            </range>
            <staticmap>
                <duid>00:01:00:01:aa:bb:cc:dd:00:11:22:33:44:55</duid>
                <ipaddrv6>fd00:1234:5678:1::10</ipaddrv6>
                <hostname>testipv6</hostname>
            </staticmap>
        </lan>
    </dhcpdv6>
    <Kea>
        <dhcp6>
            <subnets></subnets>
            <reservations></reservations>
        </dhcp6>
    </Kea>
</opnsense>
"#;

const TEST_CREATE_SUBNETS_DNSMASQ_V4: &str = r#"<?xml version="1.0"?>
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
        </opt1>
    </dhcpd>
    <dnsmasq></dnsmasq>
</opnsense>
"#;

const TEST_CREATE_SUBNETS_DNSMASQ_V6: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <lan>
            <ipaddrv6>fd00:1234:5678:1::1</ipaddrv6>
            <subnetv6>64</subnetv6>
        </lan>
    </interfaces>
    <dhcpdv6>
        <lan>
            <range>
                <from>fd00:1234:5678:1::100</from>
                <to>fd00:1234:5678:1::200</to>
            </range>
        </lan>
    </dhcpdv6>
    <dnsmasq></dnsmasq>
</opnsense>
"#;

const TEST_CREATE_OPTIONS_DNSMASQ: &str = r#"<?xml version="1.0"?>
<opnsense>
    <dhcpd>
        <opt1>
            <dnsserver>8.8.8.8</dnsserver>
            <dnsserver>1.1.1.1</dnsserver>
            <gateway>10.22.1.1</gateway>
            <domain>example.com</domain>
            <domainsearchlist>example2.com; example3.com</domainsearchlist>
            <ntpserver>10.22.1.10</ntpserver>
        </opt1>
    </dhcpd>
    <dhcpdv6>
        <lan>
            <dnsserver>fd00:1234:5678:1::1</dnsserver>
            <dnsserver>fd00:1234:5678:1::2</dnsserver>
            <domainsearchlist>example.com</domainsearchlist>
        </lan>
    </dhcpdv6>
    <dnsmasq></dnsmasq>
</opnsense>
"#;

const TEST_CREATE_OPTIONS_DNSMASQ_EXISTING: &str = r#"<?xml version="1.0"?>
<opnsense>
    <dhcpd>
        <opt1>
            <dnsserver>8.8.8.8</dnsserver>
            <dnsserver>1.1.1.1</dnsserver>
            <gateway>10.22.1.1</gateway>
            <domain>example.com</domain>
            <domainsearchlist>example2.com example3.com</domainsearchlist>
            <ntpserver>10.22.1.10</ntpserver>
        </opt1>
    </dhcpd>
    <dnsmasq>
        <dhcp_options uuid="existing-opt-1">
            <type>set</type>
            <option>6</option>
            <option6></option6>
            <interface>opt1</interface>
            <tag></tag>
            <set_tag></set_tag>
            <value>9.9.9.9</value>
            <force></force>
            <description></description>
        </dhcp_options>
    </dnsmasq>
</opnsense>
"#;

const TEST_CREATE_SUBNETS_KEA_V4_EXISTING: &str = r#"<?xml version="1.0"?>
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
                <ipaddr>10.22.1.100</ipaddr>
            </staticmap>
        </opt1>
    </dhcpd>
    <Kea>
        <dhcp4>
            <subnets>
                <subnet4 uuid="existing-subnet">
                    <subnet>10.22.1.0/24</subnet>
                    <pools>
                        <pool uuid="existing-pool">
                            <pool>10.22.1.50 - 10.22.1.60</pool>
                        </pool>
                    </pools>
                </subnet4>
            </subnets>
            <reservations></reservations>
        </dhcp4>
    </Kea>
</opnsense>
"#;

const TEST_CREATE_SUBNETS_DNSMASQ_V4_EXISTING: &str = r#"<?xml version="1.0"?>
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
        </opt1>
    </dhcpd>
    <dnsmasq>
        <dhcp_ranges uuid="existing-range">
            <interface>opt1</interface>
            <set_tag></set_tag>
            <start_addr>10.22.1.100</start_addr>
            <end_addr>10.22.1.200</end_addr>
            <subnet_mask>255.255.255.0</subnet_mask>
            <constructor></constructor>
            <mode></mode>
            <lease_time></lease_time>
            <domain_type>old</domain_type>
            <domain>old.example</domain>
            <nosync>0</nosync>
        </dhcp_ranges>
    </dnsmasq>
</opnsense>
"#;

const TEST_CREATE_SUBNETS_RANGE_OUTSIDE_CIDR: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <opt1>
            <ipaddr>10.0.0.1</ipaddr>
            <subnet>24</subnet>
        </opt1>
    </interfaces>
    <dhcpd>
        <opt1>
            <range>
                <from>10.0.1.100</from>
                <to>10.0.1.200</to>
            </range>
        </opt1>
    </dhcpd>
    <Kea>
        <dhcp4>
            <subnets></subnets>
        </dhcp4>
    </Kea>
</opnsense>
"#;

const TEST_CREATE_SUBNETS_MISSING_INTERFACE: &str = r#"<?xml version="1.0"?>
<opnsense>
    <dhcpd>
        <opt1>
            <range>
                <from>10.0.0.100</from>
                <to>10.0.0.200</to>
            </range>
        </opt1>
    </dhcpd>
    <Kea>
        <dhcp4>
            <subnets></subnets>
        </dhcp4>
    </Kea>
</opnsense>
"#;

const TEST_CREATE_SUBNETS_DHCP_INTERFACE: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <opt1>
            <ipaddr>dhcp</ipaddr>
            <subnet>24</subnet>
        </opt1>
    </interfaces>
    <dhcpd>
        <opt1>
            <range>
                <from>10.0.0.100</from>
                <to>10.0.0.200</to>
            </range>
        </opt1>
    </dhcpd>
    <Kea>
        <dhcp4>
            <subnets></subnets>
        </dhcp4>
    </Kea>
</opnsense>
"#;

const TEST_CREATE_SUBNETS_TRACK6_INTERFACE: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <lan>
            <ipaddrv6>track6</ipaddrv6>
            <subnetv6>64</subnetv6>
        </lan>
    </interfaces>
    <dhcpdv6>
        <lan>
            <range>
                <from>fd00:1234:5678:1::100</from>
                <to>fd00:1234:5678:1::200</to>
            </range>
        </lan>
    </dhcpdv6>
    <Kea>
        <dhcp6>
            <subnets></subnets>
        </dhcp6>
    </Kea>
</opnsense>
"#;

const TEST_CREATE_SUBNETS_MULTI_RANGE_V4: &str = r#"<?xml version="1.0"?>
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
                <from>10.22.1.10</from>
                <to>10.22.1.20</to>
            </range>
            <range>
                <from>10.22.1.100</from>
                <to>10.22.1.200</to>
            </range>
        </opt1>
    </dhcpd>
    <Kea>
        <dhcp4>
            <subnets></subnets>
        </dhcp4>
    </Kea>
</opnsense>
"#;

const TEST_CREATE_SUBNETS_MULTI_RANGE_V6: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <lan>
            <ipaddrv6>fd00:1234:5678:1::1</ipaddrv6>
            <subnetv6>64</subnetv6>
        </lan>
    </interfaces>
    <dhcpdv6>
        <lan>
            <range>
                <from>fd00:1234:5678:1::10</from>
                <to>fd00:1234:5678:1::20</to>
            </range>
            <range>
                <from>fd00:1234:5678:1::100</from>
                <to>fd00:1234:5678:1::200</to>
            </range>
        </lan>
    </dhcpdv6>
    <Kea>
        <dhcp6>
            <subnets></subnets>
        </dhcp6>
    </Kea>
</opnsense>
"#;

const TEST_CREATE_OPTIONS_KEA_V4: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <opt1>
            <ipaddr>10.22.1.1</ipaddr>
            <subnet>24</subnet>
        </opt1>
    </interfaces>
    <dhcpd>
        <opt1>
            <dnsserver>8.8.8.8</dnsserver>
            <dnsserver>1.1.1.1</dnsserver>
            <gateway>10.22.1.1</gateway>
            <domain>example.com</domain>
            <domainsearchlist>example2.com; example3.com</domainsearchlist>
            <ntpserver>10.22.1.10</ntpserver>
        </opt1>
    </dhcpd>
    <Kea>
        <dhcp4>
            <subnets>
                <subnet4 uuid="s4">
                    <subnet>10.22.1.0/24</subnet>
                    <option_data_autocollect>1</option_data_autocollect>
                    <option_data>
                        <domain_name_servers/>
                        <domain_search/>
                        <routers/>
                        <domain_name/>
                        <ntp_servers/>
                    </option_data>
                </subnet4>
            </subnets>
        </dhcp4>
    </Kea>
</opnsense>
"#;

const TEST_CREATE_OPTIONS_KEA_V6: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <lan>
            <ipaddrv6>fd00:1234:5678:1::1</ipaddrv6>
            <subnetv6>64</subnetv6>
        </lan>
    </interfaces>
    <dhcpdv6>
        <lan>
            <dnsserver>fd00:1234:5678:1::1</dnsserver>
            <dnsserver>fd00:1234:5678:1::2</dnsserver>
            <domainsearchlist>example.com</domainsearchlist>
        </lan>
    </dhcpdv6>
    <Kea>
        <dhcp6>
            <subnets>
                <subnet6 uuid="s6">
                    <subnet>fd00:1234:5678:1::/64</subnet>
                    <option_data>
                        <dns_servers/>
                        <domain_search/>
                    </option_data>
                </subnet6>
            </subnets>
        </dhcp6>
    </Kea>
</opnsense>
"#;

const TEST_CREATE_OPTIONS_KEA_V4_EXISTING: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <opt1>
            <ipaddr>10.22.1.1</ipaddr>
            <subnet>24</subnet>
        </opt1>
    </interfaces>
    <dhcpd>
        <opt1>
            <dnsserver>8.8.8.8</dnsserver>
            <gateway>10.22.1.1</gateway>
        </opt1>
    </dhcpd>
    <Kea>
        <dhcp4>
            <subnets>
                <subnet4 uuid="s4">
                    <subnet>10.22.1.0/24</subnet>
                    <option_data>
                        <domain_name_servers>9.9.9.9</domain_name_servers>
                        <routers>10.22.1.254</routers>
                    </option_data>
                </subnet4>
            </subnets>
        </dhcp4>
    </Kea>
</opnsense>
"#;
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
fn test_error_when_kea_not_configured() {
    let xml_no_kea = r#"<?xml version="1.0"?>
<opnsense>
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

// ---------------------------------------------------------------------------
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
fn test_create_subnets_kea_v4() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_KEA_V4);
    let mut output = Vec::new();
    let mut options = MigrationOptions::default();
    options.create_subnets = true;

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
    let pool = pools
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .find(|e| e.name == "pool")
        .expect("Should have pool");
    let pool_value = pool
        .get_child("pool")
        .and_then(|e| e.get_text())
        .expect("Should have pool value");
    assert_eq!(pool_value, "10.22.1.100 - 10.22.1.200");
}

#[test]
fn test_create_subnets_kea_v6() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_KEA_V6);
    let mut output = Vec::new();
    let mut options = MigrationOptions::default();
    options.create_subnets = true;

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
    let pool = pools
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .find(|e| e.name == "pool")
        .expect("Should have pool");
    let pool_value = pool
        .get_child("pool")
        .and_then(|e| e.get_text())
        .expect("Should have pool value");
    assert_eq!(pool_value, "fd00:1234:5678:1::100 - fd00:1234:5678:1::200");
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
    let mut options = MigrationOptions::default();
    options.create_subnets = true;

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
    let pool = pools
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .find(|e| e.name == "pool")
        .expect("Should have pool");
    let pool_value = pool
        .get_child("pool")
        .and_then(|e| e.get_text())
        .expect("Should have pool value");
    assert_eq!(pool_value, "10.22.1.50 - 10.22.1.60");
}

#[test]
fn test_create_subnets_kea_existing_force() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_KEA_V4_EXISTING);
    let mut output = Vec::new();
    let mut options = MigrationOptions::default();
    options.create_subnets = true;
    options.force_subnets = true;

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
    let pool = pools
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .find(|e| e.name == "pool")
        .expect("Should have pool");
    let pool_value = pool
        .get_child("pool")
        .and_then(|e| e.get_text())
        .expect("Should have pool value");
    assert_eq!(pool_value, "10.22.1.100 - 10.22.1.200");
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
    let mut options = MigrationOptions::default();
    options.create_subnets = true;

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
    let mut options = MigrationOptions::default();
    options.create_subnets = true;

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
    let mut options = MigrationOptions::default();
    options.create_subnets = true;

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
    let mut options = MigrationOptions::default();
    options.create_subnets = true;

    let err = convert_config(input, &mut output, &options)
        .expect_err("convert should fail for track6 interface");
    assert!(err
        .to_string()
        .contains("No interface CIDR found for DHCPv6 interface"));
}

#[test]
fn test_scan_create_subnets_kea_no_mutation() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_KEA_V4);
    let mut options = MigrationOptions::default();
    options.create_subnets = true;

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
    let mut options = MigrationOptions::default();
    options.create_subnets = true;

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
    let pool_values: Vec<String> = pools
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .filter(|e| e.name == "pool")
        .filter_map(|p| {
            p.get_child("pool")
                .and_then(|e| e.get_text())
                .map(|t| t.to_string())
        })
        .collect();
    assert_eq!(pool_values.len(), 2);
    assert!(pool_values.contains(&"10.22.1.10 - 10.22.1.20".to_string()));
    assert!(pool_values.contains(&"10.22.1.100 - 10.22.1.200".to_string()));
}

#[test]
fn test_create_subnets_multiple_ranges_v6() {
    let input = Cursor::new(TEST_CREATE_SUBNETS_MULTI_RANGE_V6);
    let mut output = Vec::new();
    let mut options = MigrationOptions::default();
    options.create_subnets = true;

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
    let pool_values: Vec<String> = pools
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .filter(|e| e.name == "pool")
        .filter_map(|p| {
            p.get_child("pool")
                .and_then(|e| e.get_text())
                .map(|t| t.to_string())
        })
        .collect();
    assert_eq!(pool_values.len(), 2);
    assert!(pool_values.contains(&"fd00:1234:5678:1::10 - fd00:1234:5678:1::20".to_string()));
    assert!(pool_values.contains(&"fd00:1234:5678:1::100 - fd00:1234:5678:1::200".to_string()));
}

#[test]
fn test_create_options_kea_v4() {
    let input = Cursor::new(TEST_CREATE_OPTIONS_KEA_V4);
    let mut output = Vec::new();
    let mut options = MigrationOptions::default();
    options.create_options = true;

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
    let mut options = MigrationOptions::default();
    options.create_options = true;

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
    let mut options = MigrationOptions::default();
    options.create_options = true;

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
    let mut options_force = MigrationOptions::default();
    options_force.create_options = true;
    options_force.force_options = true;

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
