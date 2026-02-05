#[derive(Debug, Clone)]
pub struct IscStaticMap {
    pub iface: String,
    pub mac: String,
    pub ipaddr: String,
    pub hostname: Option<String>,
    pub cid: Option<String>,
    pub descr: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IscStaticMapV6 {
    pub iface: String,
    pub duid: String,
    pub ipaddr: String,
    pub hostname: Option<String>,
    pub descr: Option<String>,
    pub domain_search: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IscDhcpOptionsV4 {
    pub iface: String,
    pub dns_servers: Vec<String>,
    pub routers: Option<String>,
    pub domain_name: Option<String>,
    pub domain_search: Option<String>,
    pub ntp_servers: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct IscDhcpOptionsV6 {
    pub iface: String,
    pub dns_servers: Vec<String>,
    pub domain_search: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IscRangeV4 {
    pub iface: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone)]
pub struct IscRangeV6 {
    pub iface: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone)]
pub struct Subnet {
    pub uuid: String,
    pub cidr: String,
    pub iface: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SubnetV6 {
    pub uuid: String,
    pub cidr: String,
    pub iface: Option<String>,
}

pub type KeaSubnet = Subnet;
pub type KeaSubnetV6 = SubnetV6;

#[derive(Debug, Default)]
pub struct MigrationStats {
    pub isc_mappings_found: usize,
    pub isc_mappings_v6_found: usize,
    pub isc_ranges_found: usize,
    pub isc_ranges_v6_found: usize,
    pub target_subnets_found: usize,
    pub target_subnets_v6_found: usize,
    pub reservations_to_create: usize,
    pub reservations_v6_to_create: usize,
    pub reservations_skipped: usize,
    pub reservations_v6_skipped: usize,
    pub interfaces_configured: Vec<String>,
    pub isc_disabled_v4: Vec<String>,
    pub isc_disabled_v6: Vec<String>,
    pub backend_enabled_v4: bool,
    pub backend_enabled_v6: bool,
}

use crate::backend::Backend;

#[derive(Debug, Clone, Default)]
pub struct MigrationOptions {
    pub fail_if_existing: bool,
    pub verbose: bool,
    pub backend: Backend,
    pub create_subnets: bool,
    pub force_subnets: bool,
    pub create_options: bool,
    pub force_options: bool,
    pub enable_backend: bool,
}
