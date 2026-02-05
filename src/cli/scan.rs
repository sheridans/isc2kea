use crate::migrate::services::isc_enabled_ifaces_v4;
use crate::migrate::services::isc_enabled_ifaces_v6;
use crate::{scan_config, scan_counts, MigrationError, MigrationOptions};
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{Cursor, Read};

use super::print_scan_stats;
use super::ScanArgs;

pub(crate) fn run_scan(args: ScanArgs) -> Result<()> {
    let mut file = File::open(&args.r#in)
        .with_context(|| format!("Failed to open input file: {}", args.r#in.display()))?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .with_context(|| format!("Failed to read input file: {}", args.r#in.display()))?;

    let options = MigrationOptions {
        fail_if_existing: args.fail_if_existing,
        verbose: args.verbose,
        backend: args.backend.clone(),
        create_subnets: args.create_subnets,
        force_subnets: args.force_subnets,
        create_options: args.create_options,
        force_options: args.force_options,
        enable_backend: args.enable_backend,
    };

    let stats = match scan_config(Cursor::new(&buffer), &options) {
        Ok(stats) => stats,
        Err(e) => {
            if let Some(migration_error) = e.downcast_ref::<MigrationError>() {
                if matches!(
                    migration_error,
                    MigrationError::BackendNotConfigured { .. }
                        | MigrationError::NoBackendSubnets { .. }
                        | MigrationError::BackendV6NotConfigured { .. }
                        | MigrationError::NoBackendSubnetsV6 { .. }
                ) {
                    if let Ok(stats) = scan_counts(Cursor::new(&buffer), &args.backend) {
                        print_scan_stats(&stats, &args.backend);
                    }
                }
            }

            return Err(e);
        }
    };

    if args.verbose {
        if let Ok(root) = xmltree::Element::parse(Cursor::new(&buffer)) {
            let ifaces_v4 = isc_enabled_ifaces_v4(&root);
            let ifaces_v6 = isc_enabled_ifaces_v6(&root);
            if !ifaces_v4.is_empty() {
                println!("ISC DHCP enabled interfaces (v4): {}", ifaces_v4.join(", "));
            }
            if !ifaces_v6.is_empty() {
                println!("ISC DHCP enabled interfaces (v6): {}", ifaces_v6.join(", "));
            }
        }
    }

    print_scan_stats(&stats, &args.backend);
    Ok(())
}
