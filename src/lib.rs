mod errors;
mod extract;
mod migrate;
mod migrate_v4;
mod migrate_v6;
mod subnet;
mod types;
mod xml_helpers;

pub use errors::MigrationError;
pub use extract::{
    extract_existing_reservation_duids_v6, extract_existing_reservation_ips,
    extract_existing_reservation_ips_v6, extract_isc_mappings, extract_isc_mappings_v6,
    extract_kea_subnets, extract_kea_subnets_v6,
};
pub use migrate::{convert_config, scan_config, scan_counts};
pub use subnet::{find_subnet_for_ip, find_subnet_for_ip_v6, ip_in_subnet, ip_in_subnet_v6};
pub use types::{
    IscStaticMap, IscStaticMapV6, KeaSubnet, KeaSubnetV6, MigrationOptions, MigrationStats,
};
