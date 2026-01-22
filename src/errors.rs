use thiserror::Error;

#[derive(Error, Debug)]
pub enum MigrationError {
    #[error("IP address {0} does not match any Kea subnet")]
    NoMatchingSubnet(String),

    #[error("Invalid IP address: {0}")]
    InvalidIpAddress(String),

    #[error("Invalid CIDR notation: {0}")]
    InvalidCidr(String),

    #[error("Kea DHCPv4 not configured in config.xml. Please configure Kea subnets first.")]
    KeaNotConfigured,

    #[error("No Kea subnets found. Please configure at least one Kea subnet before migration.")]
    NoKeaSubnets,

    #[error("Kea DHCPv6 not configured in config.xml. Please configure Kea DHCPv6 first.")]
    KeaV6NotConfigured,

    #[error(
        "No Kea DHCPv6 subnets found. Please configure at least one Kea DHCPv6 subnet before \
         migration."
    )]
    NoKeaSubnetsV6,
}
