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
pub struct KeaSubnet {
    pub uuid: String,
    pub cidr: String,
}

#[derive(Debug, Clone)]
pub struct KeaSubnetV6 {
    pub uuid: String,
    pub cidr: String,
}

#[derive(Debug)]
pub struct MigrationStats {
    pub isc_mappings_found: usize,
    pub isc_mappings_v6_found: usize,
    pub kea_subnets_found: usize,
    pub kea_subnets_v6_found: usize,
    pub reservations_to_create: usize,
    pub reservations_v6_to_create: usize,
    pub reservations_skipped: usize,
    pub reservations_v6_skipped: usize,
}

#[derive(Debug, Clone, Default)]
pub struct MigrationOptions {
    pub fail_if_existing: bool,
    pub verbose: bool,
}
