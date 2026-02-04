use anyhow::Result;
use std::collections::HashMap;

use crate::subnet::{iface_for_ip, iface_for_ip_v6};
use crate::{IscStaticMap, IscStaticMapV6, MigrationError};

pub(crate) fn short_uuid(uuid: &str) -> &str {
    uuid.get(..8).unwrap_or(uuid)
}

pub(crate) fn validate_mapping_ifaces_v4(
    mappings: &[IscStaticMap],
    iface_cidrs: &HashMap<String, String>,
) -> Result<()> {
    for mapping in mappings {
        let derived = iface_for_ip(&mapping.ipaddr, iface_cidrs)?;
        if !derived.eq_ignore_ascii_case(&mapping.iface) {
            return Err(MigrationError::InterfaceMismatch {
                ip: mapping.ipaddr.clone(),
                isc_iface: mapping.iface.clone(),
                derived_iface: derived,
            }
            .into());
        }
    }
    Ok(())
}

pub(crate) fn validate_mapping_ifaces_v6(
    mappings: &[IscStaticMapV6],
    iface_cidrs: &HashMap<String, String>,
) -> Result<()> {
    for mapping in mappings {
        let derived = iface_for_ip_v6(&mapping.ipaddr, iface_cidrs)?;
        if !derived.eq_ignore_ascii_case(&mapping.iface) {
            return Err(MigrationError::InterfaceMismatch {
                ip: mapping.ipaddr.clone(),
                isc_iface: mapping.iface.clone(),
                derived_iface: derived,
            }
            .into());
        }
    }
    Ok(())
}
