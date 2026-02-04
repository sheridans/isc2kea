pub mod backend;
pub mod cli;
mod errors;
mod extract;
mod extract_dnsmasq;
mod migrate;
mod migrate_dnsmasq;
mod migrate_v4;
mod migrate_v6;
mod subnet;
mod types;
mod xml_helpers;

pub use backend::Backend;
pub use errors::MigrationError;
pub use extract::{
    extract_existing_reservation_duids_v6, extract_existing_reservation_ips,
    extract_existing_reservation_ips_v6, extract_isc_mappings, extract_isc_mappings_v6,
    extract_isc_options_v4, extract_isc_options_v6, extract_kea_subnets, extract_kea_subnets_v6,
};
pub use migrate::{convert_config, scan_config, scan_counts};
pub use subnet::{
    find_subnet_for_ip, find_subnet_for_ip_v6, ip_in_subnet, ip_in_subnet_v6, prefix_to_netmask,
};
pub use types::{
    IscDhcpOptionsV4, IscDhcpOptionsV6, IscRangeV4, IscRangeV6, IscStaticMap, IscStaticMapV6,
    KeaSubnet, KeaSubnetV6, MigrationOptions, MigrationStats, Subnet, SubnetV6,
};
