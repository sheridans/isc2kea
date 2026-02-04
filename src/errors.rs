use thiserror::Error;

#[derive(Error, Debug)]
pub enum MigrationError {
    #[error("IP address {0} does not match any configured subnet")]
    NoMatchingSubnet(String),

    #[error("Invalid IP address: {0}")]
    InvalidIpAddress(String),

    #[error("Invalid CIDR notation: {0}")]
    InvalidCidr(String),

    #[error("IP address {0} does not match any configured interface subnet")]
    NoMatchingInterface(String),

    #[error(
        "IP address {ip} maps to interface {derived_iface} but ISC mapping is under interface {isc_iface}"
    )]
    InterfaceMismatch {
        ip: String,
        isc_iface: String,
        derived_iface: String,
    },

    #[error("{backend} DHCPv4 not configured in config.xml. Please configure {backend} first.")]
    BackendNotConfigured { backend: String },

    #[error(
        "No {backend} subnets found. Please configure at least one {backend} subnet before \
         migration."
    )]
    NoBackendSubnets { backend: String },

    #[error(
        "{backend} DHCPv6 not configured in config.xml. Please configure {backend} DHCPv6 first."
    )]
    BackendV6NotConfigured { backend: String },

    #[error(
        "No {backend} DHCPv6 subnets found. Please configure at least one {backend} DHCPv6 \
         subnet before migration."
    )]
    NoBackendSubnetsV6 { backend: String },
}
