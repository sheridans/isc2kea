mod interfaces;
mod isc;
mod kea;

pub use interfaces::{extract_interface_cidrs, extract_interface_cidrs_v6};
pub use isc::{
    extract_isc_mappings, extract_isc_mappings_v6, extract_isc_options_v4, extract_isc_options_v6,
    extract_isc_ranges, extract_isc_ranges_v6,
};
pub use kea::{
    extract_existing_reservation_duids_v6, extract_existing_reservation_ips,
    extract_existing_reservation_ips_v6, extract_kea_subnets, extract_kea_subnets_v6,
    has_kea_dhcp4, has_kea_dhcp6,
};
