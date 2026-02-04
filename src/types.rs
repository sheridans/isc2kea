#[derive(Debug, Clone)]
pub struct IscStaticMap {
    pub mac: String,
    pub ipaddr: String,
    pub hostname: Option<String>,
    pub cid: Option<String>,
    pub descr: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IscStaticMapV6 {
    pub duid: String,
    pub ipaddr: String,
    pub hostname: Option<String>,
    pub descr: Option<String>,
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
}

#[derive(Debug, Clone)]
pub struct SubnetV6 {
    pub uuid: String,
    pub cidr: String,
}

pub type KeaSubnet = Subnet;
pub type KeaSubnetV6 = SubnetV6;

#[derive(Debug)]
pub struct MigrationStats {
    pub isc_mappings_found: usize,
    pub isc_mappings_v6_found: usize,
    pub target_subnets_found: usize,
    pub target_subnets_v6_found: usize,
    pub reservations_to_create: usize,
    pub reservations_v6_to_create: usize,
    pub reservations_skipped: usize,
    pub reservations_v6_skipped: usize,
}

use crate::backend::Backend;

#[derive(Debug, Clone, Default)]
pub struct MigrationOptions {
    pub fail_if_existing: bool,
    pub verbose: bool,
    pub backend: Backend,
    pub create_subnets: bool,
    pub force_subnets: bool,
}
