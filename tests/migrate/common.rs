use isc2kea::{Backend, MigrationOptions};
use xmltree::Element;

pub fn find_descendant_ci<'a>(elem: &'a Element, name: &str) -> Option<&'a Element> {
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

pub fn reservation_fields(reservation: &Element) -> Vec<(String, String)> {
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

pub fn reservations_as_fields(dhcp: &Element) -> Vec<Vec<(String, String)>> {
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
pub fn dnsmasq_host_fields(host: &Element) -> Vec<(String, String)> {
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

pub fn dnsmasq_hosts_as_fields(dnsmasq: &Element) -> Vec<Vec<(String, String)>> {
    dnsmasq
        .children
        .iter()
        .filter_map(|child| child.as_element())
        .filter(|elem| elem.name == "hosts")
        .map(dnsmasq_host_fields)
        .collect()
}

pub fn dnsmasq_hosts(root: &Element) -> Vec<&Element> {
    let dnsmasq = find_descendant_ci(root, "dnsmasq").expect("Should have dnsmasq node");
    dnsmasq
        .children
        .iter()
        .filter_map(|c| c.as_element())
        .filter(|e| e.name == "hosts")
        .collect()
}

pub fn dnsmasq_option_value(
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

pub fn dnsmasq_options() -> MigrationOptions {
    MigrationOptions {
        backend: Backend::Dnsmasq,
        ..Default::default()
    }
}

pub fn dnsmasq_options_create_subnets() -> MigrationOptions {
    MigrationOptions {
        backend: Backend::Dnsmasq,
        create_subnets: true,
        ..Default::default()
    }
}

pub const TEST_XML: &str = r#"<?xml version="1.0"?>
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

pub const TEST_XML_V6: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <opt2>
            <ipaddrv6>2001:db8:42::1</ipaddrv6>
            <subnetv6>64</subnetv6>
        </opt2>
    </interfaces>
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

pub const TEST_DNSMASQ_XML: &str = r#"<?xml version="1.0"?>
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
                <descr>Test Server</descr>
            </staticmap>
        </lan>
    </dhcpd>
    <dnsmasq>
    </dnsmasq>
</opnsense>
"#;

pub const TEST_DNSMASQ_XML_WITH_EXISTING_IP: &str = r#"<?xml version="1.0"?>
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
    <dnsmasq>
        <hosts uuid="existing-host-1">
            <hwaddr>99:99:99:99:99:99</hwaddr>
            <ip>192.168.1.10</ip>
            <host>existing</host>
        </hosts>
    </dnsmasq>
</opnsense>
"#;

pub const TEST_DNSMASQ_XML_WITH_EXISTING_MAC: &str = r#"<?xml version="1.0"?>
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
    <dnsmasq>
        <hosts uuid="existing-host-1">
            <hwaddr>00:11:22:33:44:55</hwaddr>
            <ip>192.168.1.99</ip>
            <host>existing</host>
        </hosts>
    </dnsmasq>
</opnsense>
"#;

pub const TEST_DNSMASQ_XML_V6: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <lan>
            <ipaddrv6>fd00:1234:5678:1::1</ipaddrv6>
            <subnetv6>64</subnetv6>
        </lan>
    </interfaces>
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

pub const TEST_DNSMASQ_XML_V6_WITH_EXISTING_IP: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <lan>
            <ipaddrv6>fd00:1234:5678:1::1</ipaddrv6>
            <subnetv6>64</subnetv6>
        </lan>
    </interfaces>
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

pub const TEST_DNSMASQ_XML_V6_WITH_EXISTING_DUID: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <lan>
            <ipaddrv6>fd00:1234:5678:1::1</ipaddrv6>
            <subnetv6>64</subnetv6>
        </lan>
    </interfaces>
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

pub const TEST_DNSMASQ_XML_V6_WITH_EXISTING_CLIENT_ID_ONLY: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <lan>
            <ipaddrv6>fd00:1234:5678:1::1</ipaddrv6>
            <subnetv6>64</subnetv6>
        </lan>
    </interfaces>
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
pub const TEST_XML_V6_WITH_EXISTING_DUID: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <opt2>
            <ipaddrv6>2001:db8:42::1</ipaddrv6>
            <subnetv6>64</subnetv6>
        </opt2>
    </interfaces>
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

pub const TEST_CREATE_SUBNETS_KEA_V4: &str = r#"<?xml version="1.0"?>
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

pub const TEST_CREATE_SUBNETS_KEA_V6: &str = r#"<?xml version="1.0"?>
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

pub const TEST_CREATE_SUBNETS_DNSMASQ_V4: &str = r#"<?xml version="1.0"?>
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

pub const TEST_CREATE_SUBNETS_DNSMASQ_V6: &str = r#"<?xml version="1.0"?>
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

pub const TEST_CREATE_OPTIONS_DNSMASQ: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <lan>
            <ipaddr>192.168.1.1</ipaddr>
            <subnet>24</subnet>
        </lan>
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

pub const TEST_CREATE_OPTIONS_DNSMASQ_EXISTING: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <lan>
            <ipaddr>192.168.1.1</ipaddr>
            <subnet>24</subnet>
        </lan>
    </interfaces>
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

pub const TEST_CREATE_SUBNETS_KEA_V4_EXISTING: &str = r#"<?xml version="1.0"?>
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
                    <pools>10.22.1.50-10.22.1.60</pools>
                </subnet4>
            </subnets>
            <reservations></reservations>
        </dhcp4>
    </Kea>
</opnsense>
"#;

pub const TEST_CREATE_SUBNETS_DNSMASQ_V4_EXISTING: &str = r#"<?xml version="1.0"?>
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

pub const TEST_CREATE_SUBNETS_RANGE_OUTSIDE_CIDR: &str = r#"<?xml version="1.0"?>
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

pub const TEST_CREATE_SUBNETS_MISSING_INTERFACE: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <lan>
            <ipaddr>192.168.1.1</ipaddr>
            <subnet>24</subnet>
        </lan>
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

pub const TEST_CREATE_SUBNETS_DHCP_INTERFACE: &str = r#"<?xml version="1.0"?>
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

pub const TEST_CREATE_SUBNETS_TRACK6_INTERFACE: &str = r#"<?xml version="1.0"?>
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

pub const TEST_CREATE_SUBNETS_MULTI_RANGE_V4: &str = r#"<?xml version="1.0"?>
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

pub const TEST_CREATE_SUBNETS_MULTI_RANGE_V6: &str = r#"<?xml version="1.0"?>
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

pub const TEST_CREATE_OPTIONS_KEA_V4: &str = r#"<?xml version="1.0"?>
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

pub const TEST_CREATE_OPTIONS_KEA_V6: &str = r#"<?xml version="1.0"?>
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

pub const TEST_CREATE_OPTIONS_KEA_V4_EXISTING: &str = r#"<?xml version="1.0"?>
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

pub const TEST_CREATE_SUBNETS_KEA_V4_EXISTING_INTERFACES: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <opt1>
            <ipaddr>10.22.1.1</ipaddr>
            <subnet>24</subnet>
        </opt1>
        <opt2>
            <ipaddr>10.22.2.1</ipaddr>
            <subnet>24</subnet>
        </opt2>
    </interfaces>
    <dhcpd>
        <opt2>
            <range>
                <from>10.22.2.100</from>
                <to>10.22.2.200</to>
            </range>
            <staticmap>
                <mac>04:d9:f5:cb:9b:54</mac>
                <ipaddr>10.22.2.50</ipaddr>
            </staticmap>
        </opt2>
    </dhcpd>
    <Kea>
        <dhcp4>
            <general>
                <interfaces>opt1</interfaces>
            </general>
            <subnets></subnets>
            <reservations></reservations>
        </dhcp4>
    </Kea>
</opnsense>
"#;

pub const TEST_CREATE_SUBNETS_DNSMASQ_V4_EXISTING_INTERFACES: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <opt1>
            <ipaddr>10.22.1.1</ipaddr>
            <subnet>24</subnet>
        </opt1>
        <opt2>
            <ipaddr>10.22.2.1</ipaddr>
            <subnet>24</subnet>
        </opt2>
    </interfaces>
    <dhcpd>
        <opt2>
            <range>
                <from>10.22.2.100</from>
                <to>10.22.2.200</to>
            </range>
        </opt2>
    </dhcpd>
    <dnsmasq>
        <interface>opt1</interface>
    </dnsmasq>
</opnsense>
"#;

pub const TEST_ENABLE_BACKEND_KEA: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <opt1>
            <ipaddr>10.22.1.1</ipaddr>
            <subnet>24</subnet>
        </opt1>
    </interfaces>
    <dhcpd>
        <opt1>
            <enable>1</enable>
            <range>
                <from>10.22.1.100</from>
                <to>10.22.1.200</to>
            </range>
        </opt1>
    </dhcpd>
    <Kea>
        <dhcp4>
            <general>
                <enabled>0</enabled>
            </general>
            <subnets></subnets>
            <reservations></reservations>
        </dhcp4>
    </Kea>
</opnsense>
"#;

pub const TEST_ENABLE_BACKEND_DNSMASQ: &str = r#"<?xml version="1.0"?>
<opnsense>
    <interfaces>
        <opt1>
            <ipaddr>10.22.1.1</ipaddr>
            <subnet>24</subnet>
        </opt1>
    </interfaces>
    <dhcpd>
        <opt1>
            <enable>1</enable>
            <range>
                <from>10.22.1.100</from>
                <to>10.22.1.200</to>
            </range>
        </opt1>
    </dhcpd>
    <dnsmasq>
        <enable>0</enable>
    </dnsmasq>
</opnsense>
"#;
