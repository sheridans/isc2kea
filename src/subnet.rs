use anyhow::Result;
use ipnet::{Ipv4Net, Ipv6Net};
use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use crate::{MigrationError, Subnet, SubnetV6};

/// Check if an IP address is contained within a CIDR subnet
pub fn ip_in_subnet(ip: &str, cidr: &str) -> Result<bool> {
    let ip_addr =
        Ipv4Addr::from_str(ip).map_err(|_| MigrationError::InvalidIpAddress(ip.to_string()))?;

    let network =
        Ipv4Net::from_str(cidr).map_err(|_| MigrationError::InvalidCidr(cidr.to_string()))?;

    Ok(network.contains(&ip_addr))
}

/// Find the matching subnet UUID for an IP address
pub fn find_subnet_for_ip(ip: &str, subnets: &[Subnet]) -> Result<String> {
    let ip_addr =
        Ipv4Addr::from_str(ip).map_err(|_| MigrationError::InvalidIpAddress(ip.to_string()))?;

    let mut parsed = Vec::with_capacity(subnets.len());
    for subnet in subnets {
        let net = Ipv4Net::from_str(&subnet.cidr)
            .map_err(|_| MigrationError::InvalidCidr(subnet.cidr.to_string()))?;
        parsed.push((net.prefix_len(), subnet, net));
    }

    parsed.sort_by(|a, b| b.0.cmp(&a.0));

    for (_, subnet, net) in parsed {
        if net.contains(&ip_addr) {
            return Ok(subnet.uuid.clone());
        }
    }

    Err(MigrationError::NoMatchingSubnet(ip.to_string()).into())
}

/// Find the interface name for an IPv4 address based on interface CIDRs
pub fn iface_for_ip(ip: &str, iface_cidrs: &HashMap<String, String>) -> Result<String> {
    let ip_addr =
        Ipv4Addr::from_str(ip).map_err(|_| MigrationError::InvalidIpAddress(ip.to_string()))?;

    let mut parsed = Vec::with_capacity(iface_cidrs.len());
    for (iface, cidr) in iface_cidrs {
        let net =
            Ipv4Net::from_str(cidr).map_err(|_| MigrationError::InvalidCidr(cidr.to_string()))?;
        parsed.push((net.prefix_len(), iface, net));
    }

    parsed.sort_by(|a, b| b.0.cmp(&a.0));

    for (_, iface, net) in parsed {
        if net.contains(&ip_addr) {
            return Ok(iface.clone());
        }
    }

    Err(MigrationError::NoMatchingInterface(ip.to_string()).into())
}

/// Check if an IPv6 address is contained within a CIDR subnet
pub fn ip_in_subnet_v6(ip: &str, cidr: &str) -> Result<bool> {
    let ip_addr =
        Ipv6Addr::from_str(ip).map_err(|_| MigrationError::InvalidIpAddress(ip.to_string()))?;

    let network =
        Ipv6Net::from_str(cidr).map_err(|_| MigrationError::InvalidCidr(cidr.to_string()))?;

    Ok(network.contains(&ip_addr))
}

/// Convert an IPv4 prefix length to a subnet mask string (e.g. 24 -> 255.255.255.0)
pub fn prefix_to_netmask(prefix: u8) -> Result<String> {
    let net = Ipv4Net::new(Ipv4Addr::UNSPECIFIED, prefix)
        .map_err(|_| MigrationError::InvalidCidr(prefix.to_string()))?;
    Ok(net.netmask().to_string())
}

/// Find the matching IPv6 subnet UUID for an IP address
pub fn find_subnet_for_ip_v6(ip: &str, subnets: &[SubnetV6]) -> Result<String> {
    let ip_addr =
        Ipv6Addr::from_str(ip).map_err(|_| MigrationError::InvalidIpAddress(ip.to_string()))?;

    let mut parsed = Vec::with_capacity(subnets.len());
    for subnet in subnets {
        let net = Ipv6Net::from_str(&subnet.cidr)
            .map_err(|_| MigrationError::InvalidCidr(subnet.cidr.to_string()))?;
        parsed.push((net.prefix_len(), subnet, net));
    }

    parsed.sort_by(|a, b| b.0.cmp(&a.0));

    for (_, subnet, net) in parsed {
        if net.contains(&ip_addr) {
            return Ok(subnet.uuid.clone());
        }
    }

    Err(MigrationError::NoMatchingSubnet(ip.to_string()).into())
}

/// Find the interface name for an IPv6 address based on interface CIDRs
pub fn iface_for_ip_v6(ip: &str, iface_cidrs: &HashMap<String, String>) -> Result<String> {
    let ip_addr =
        Ipv6Addr::from_str(ip).map_err(|_| MigrationError::InvalidIpAddress(ip.to_string()))?;

    let mut parsed = Vec::with_capacity(iface_cidrs.len());
    for (iface, cidr) in iface_cidrs {
        let net =
            Ipv6Net::from_str(cidr).map_err(|_| MigrationError::InvalidCidr(cidr.to_string()))?;
        parsed.push((net.prefix_len(), iface, net));
    }

    parsed.sort_by(|a, b| b.0.cmp(&a.0));

    for (_, iface, net) in parsed {
        if net.contains(&ip_addr) {
            return Ok(iface.clone());
        }
    }

    Err(MigrationError::NoMatchingInterface(ip.to_string()).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_in_subnet() {
        assert!(ip_in_subnet("192.168.1.10", "192.168.1.0/24").unwrap());
        assert!(ip_in_subnet("192.168.1.254", "192.168.1.0/24").unwrap());
        assert!(!ip_in_subnet("192.168.2.10", "192.168.1.0/24").unwrap());
        assert!(!ip_in_subnet("10.0.0.1", "192.168.1.0/24").unwrap());

        // Test /16
        assert!(ip_in_subnet("10.20.30.40", "10.20.0.0/16").unwrap());
        assert!(!ip_in_subnet("10.21.30.40", "10.20.0.0/16").unwrap());

        // Test /32 (single host)
        assert!(ip_in_subnet("192.168.1.100", "192.168.1.100/32").unwrap());
        assert!(!ip_in_subnet("192.168.1.101", "192.168.1.100/32").unwrap());
    }

    #[test]
    fn test_find_subnet_for_ip() {
        let subnets = vec![
            Subnet {
                uuid: "subnet-1".to_string(),
                cidr: "192.168.1.0/24".to_string(),
                iface: None,
            },
            Subnet {
                uuid: "subnet-2".to_string(),
                cidr: "10.0.0.0/8".to_string(),
                iface: None,
            },
        ];

        assert_eq!(
            find_subnet_for_ip("192.168.1.50", &subnets).unwrap(),
            "subnet-1"
        );
        assert_eq!(
            find_subnet_for_ip("10.20.30.40", &subnets).unwrap(),
            "subnet-2"
        );

        // Should fail for non-matching IP
        let result = find_subnet_for_ip("172.16.0.1", &subnets);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("does not match any configured subnet"));
    }

    #[test]
    fn test_find_subnet_for_ip_most_specific() {
        let subnets = vec![
            Subnet {
                uuid: "subnet-wide".to_string(),
                cidr: "10.0.0.0/16".to_string(),
                iface: None,
            },
            Subnet {
                uuid: "subnet-narrow".to_string(),
                cidr: "10.0.1.0/24".to_string(),
                iface: None,
            },
        ];

        assert_eq!(
            find_subnet_for_ip("10.0.1.42", &subnets).unwrap(),
            "subnet-narrow"
        );
    }

    #[test]
    fn test_ip_in_subnet_v6() {
        assert!(ip_in_subnet_v6("2001:db8::1", "2001:db8::/64").unwrap());
        assert!(!ip_in_subnet_v6("2001:db8:1::1", "2001:db8::/64").unwrap());
        assert!(ip_in_subnet_v6("2001:db8::1", "2001:db8::1/128").unwrap());
        assert!(!ip_in_subnet_v6("2001:db8::2", "2001:db8::1/128").unwrap());
    }

    #[test]
    fn test_find_subnet_for_ip_v6() {
        let subnets = vec![
            SubnetV6 {
                uuid: "subnet-6a".to_string(),
                cidr: "2001:db8:42::/64".to_string(),
                iface: None,
            },
            SubnetV6 {
                uuid: "subnet-6b".to_string(),
                cidr: "fd00:abcd::/64".to_string(),
                iface: None,
            },
        ];

        assert_eq!(
            find_subnet_for_ip_v6("2001:db8:42::10", &subnets).unwrap(),
            "subnet-6a"
        );
        assert_eq!(
            find_subnet_for_ip_v6("fd00:abcd::1", &subnets).unwrap(),
            "subnet-6b"
        );

        let result = find_subnet_for_ip_v6("2001:db8:99::1", &subnets);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("does not match any configured subnet"));
    }

    #[test]
    fn test_find_subnet_for_ip_v6_most_specific() {
        let subnets = vec![
            SubnetV6 {
                uuid: "subnet-wide".to_string(),
                cidr: "2001:db8::/48".to_string(),
                iface: None,
            },
            SubnetV6 {
                uuid: "subnet-narrow".to_string(),
                cidr: "2001:db8:abcd::/64".to_string(),
                iface: None,
            },
        ];

        assert_eq!(
            find_subnet_for_ip_v6("2001:db8:abcd::42", &subnets).unwrap(),
            "subnet-narrow"
        );
    }

    #[test]
    fn test_iface_for_ip() {
        let mut iface_cidrs = HashMap::new();
        iface_cidrs.insert("lan".to_string(), "192.168.1.0/24".to_string());
        iface_cidrs.insert("opt1".to_string(), "10.0.0.0/8".to_string());

        assert_eq!(iface_for_ip("192.168.1.42", &iface_cidrs).unwrap(), "lan");
        assert_eq!(iface_for_ip("10.20.30.40", &iface_cidrs).unwrap(), "opt1");
        assert!(iface_for_ip("172.16.0.1", &iface_cidrs).is_err());
    }

    #[test]
    fn test_iface_for_ip_v6() {
        let mut iface_cidrs = HashMap::new();
        iface_cidrs.insert("lan".to_string(), "fd00:abcd::/64".to_string());
        iface_cidrs.insert("opt1".to_string(), "2001:db8:42::/64".to_string());

        assert_eq!(
            iface_for_ip_v6("fd00:abcd::1", &iface_cidrs).unwrap(),
            "lan"
        );
        assert_eq!(
            iface_for_ip_v6("2001:db8:42::10", &iface_cidrs).unwrap(),
            "opt1"
        );
        assert!(iface_for_ip_v6("2001:db8:99::1", &iface_cidrs).is_err());
    }
}
