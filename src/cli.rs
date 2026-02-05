use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::Cursor;
use std::io::{self, Read};
use std::path::PathBuf;

use crate::{
    convert_config, scan_config, scan_counts, Backend, MigrationError, MigrationOptions,
    MigrationStats,
};

#[derive(Parser)]
#[command(
    name = "isc2kea",
    about = "Migrate ISC DHCP static mappings to Kea/dnsmasq DHCP configurations",
    long_about = "Designed for OPNsense config.xml but may work with similar XML schemas."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan configuration and show migration statistics (read-only)
    Scan {
        /// Input config.xml file path
        #[arg(short, long, default_value = "/conf/config.xml")]
        r#in: PathBuf,

        /// Target DHCP backend
        #[arg(short, long, value_enum, default_value_t = Backend::Kea)]
        backend: Backend,

        /// Abort if any existing reservations/hosts are found
        #[arg(long)]
        fail_if_existing: bool,

        /// Create missing subnets/ranges in the target backend
        #[arg(long)]
        create_subnets: bool,

        /// Overwrite existing subnets/ranges when creating them
        #[arg(long, requires = "create_subnets")]
        force_subnets: bool,

        /// Create DHCP options in the target backend
        #[arg(long)]
        create_options: bool,

        /// Overwrite existing DHCP options when creating them
        #[arg(long, requires = "create_options")]
        force_options: bool,

        /// Enable target backend and disable ISC DHCP on migrated interfaces
        #[arg(long)]
        enable_backend: bool,

        /// Show detailed progress for each mapping
        #[arg(short, long)]
        verbose: bool,
    },

    /// Convert ISC mappings to target backend format and write to output file
    Convert {
        /// Input config.xml file path
        #[arg(short, long, default_value = "/conf/config.xml")]
        r#in: PathBuf,

        /// Target DHCP backend
        #[arg(short, long, value_enum, default_value_t = Backend::Kea)]
        backend: Backend,

        /// Output file path for converted XML
        #[arg(short, long)]
        out: PathBuf,

        /// Abort if any existing reservations/hosts are found
        #[arg(long)]
        fail_if_existing: bool,

        /// Create missing subnets/ranges in the target backend
        #[arg(long)]
        create_subnets: bool,

        /// Overwrite existing subnets/ranges when creating them
        #[arg(long, requires = "create_subnets")]
        force_subnets: bool,

        /// Create DHCP options in the target backend
        #[arg(long)]
        create_options: bool,

        /// Overwrite existing DHCP options when creating them
        #[arg(long, requires = "create_options")]
        force_options: bool,

        /// Enable target backend and disable ISC DHCP on migrated interfaces
        #[arg(long)]
        enable_backend: bool,

        /// Show detailed progress for each mapping
        #[arg(short, long)]
        verbose: bool,

        /// Overwrite output file if it exists
        #[arg(long)]
        force: bool,
    },
}

pub fn run_with_args<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::parse_from(args);

    match cli.command {
        Commands::Scan {
            r#in,
            backend,
            fail_if_existing,
            create_subnets,
            force_subnets,
            create_options,
            force_options,
            enable_backend,
            verbose,
        } => {
            let mut file = File::open(&r#in)
                .with_context(|| format!("Failed to open input file: {}", r#in.display()))?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .with_context(|| format!("Failed to read input file: {}", r#in.display()))?;

            let options = MigrationOptions {
                fail_if_existing,
                verbose,
                backend: backend.clone(),
                create_subnets,
                force_subnets,
                create_options,
                force_options,
                enable_backend,
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
                            if let Ok(stats) = scan_counts(Cursor::new(&buffer), &backend) {
                                print_scan_stats(&stats, &backend);
                            }
                        }
                    }

                    return Err(e);
                }
            };

            print_scan_stats(&stats, &backend);
        }

        Commands::Convert {
            r#in,
            backend,
            out,
            fail_if_existing,
            create_subnets,
            force_subnets,
            create_options,
            force_options,
            enable_backend,
            verbose,
            force,
        } => {
            // Critical safety check: prevent input == output
            let in_canonical = std::fs::canonicalize(&r#in).unwrap_or_else(|_| r#in.clone());
            let (out_canonical, out_missing) = match std::fs::canonicalize(&out) {
                Ok(path) => (path, false),
                Err(e) => (out.clone(), e.kind() == io::ErrorKind::NotFound),
            };

            if in_canonical == out_canonical {
                bail!(
                    concat!(
                        "Output path must be different from input path (refusing to overwrite input).\n",
                        "Input:  {}\n",
                        "Output: {}"
                    ),
                    in_canonical.display(),
                    out_canonical.display()
                );
            }
            if out_missing {
                if let (Some(parent), Some(file_name)) = (out.parent(), out.file_name()) {
                    if let Ok(parent_canonical) = std::fs::canonicalize(parent) {
                        let reconstructed_out = parent_canonical.join(file_name);
                        if reconstructed_out == in_canonical {
                            bail!(
                                concat!(
                                    "Output path must be different from input path (refusing to overwrite input).\n",
                                    "Input:  {}\n",
                                    "Output: {}"
                                ),
                                in_canonical.display(),
                                reconstructed_out.display()
                            );
                        }
                    }
                }
            }

            let input_file = File::open(&r#in)
                .with_context(|| format!("Failed to open input file: {}", r#in.display()))?;

            if !force && out.exists() {
                bail!(
                    "Output file already exists: {} (use --force to overwrite)",
                    out.display()
                );
            }

            let tmp_path = out.with_extension(format!(
                "tmp.{}",
                std::process::id()
            ));
            let mut tmp_file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp_path)
                .with_context(|| {
                    format!(
                        "Failed to create temporary output file: {}",
                        tmp_path.display()
                    )
                })?;

            let options = MigrationOptions {
                fail_if_existing,
                verbose,
                backend: backend.clone(),
                create_subnets,
                force_subnets,
                create_options,
                force_options,
                enable_backend,
            };

            let stats = match convert_config(input_file, &mut tmp_file, &options) {
                Ok(stats) => stats,
                Err(e) => {
                    let _ = std::fs::remove_file(&tmp_path);
                    return Err(e);
                }
            };

            if let Err(e) = tmp_file.sync_all() {
                let _ = std::fs::remove_file(&tmp_path);
                return Err(e).with_context(|| {
                    format!(
                        "Failed to sync temporary output file: {}",
                        tmp_path.display()
                    )
                });
            }

            if force && out.exists() {
                std::fs::remove_file(&out).with_context(|| {
                    format!("Failed to remove existing output file: {}", out.display())
                })?;
            }

            std::fs::rename(&tmp_path, &out).with_context(|| {
                format!(
                    "Failed to replace output file: {}",
                    out.display()
                )
            })?;

            println!("\nMigration completed successfully!");
            print_convert_stats(&stats, &backend);
            println!("Output written to: {}", out.display());
        }
    }

    Ok(())
}

fn print_scan_stats(stats: &MigrationStats, backend: &Backend) {
    println!(
        "ISC DHCP static mappings found: {}",
        stats.isc_mappings_found
    );
    println!(
        "ISC DHCPv6 static mappings found: {}",
        stats.isc_mappings_v6_found
    );
    println!(
        "{} subnet4 entries found: {}",
        backend, stats.target_subnets_found
    );
    println!(
        "{} subnet6 entries found: {}",
        backend, stats.target_subnets_v6_found
    );
    println!(
        "Reservations that would be created: {}",
        stats.reservations_to_create
    );
    println!(
        "Reservations (v6) that would be created: {}",
        stats.reservations_v6_to_create
    );
    println!(
        "Reservations skipped (already exist): {}",
        stats.reservations_skipped
    );
    println!(
        "Reservations skipped (v6): {}",
        stats.reservations_v6_skipped
    );

    if !stats.interfaces_configured.is_empty() {
        println!(
            "Interfaces configured: {}",
            stats.interfaces_configured.join(", ")
        );
    }
    if !stats.isc_disabled_v4.is_empty() {
        println!(
            "ISC DHCP disabled (v4): {}",
            stats.isc_disabled_v4.join(", ")
        );
    }
    if !stats.isc_disabled_v6.is_empty() {
        println!(
            "ISC DHCP disabled (v6): {}",
            stats.isc_disabled_v6.join(", ")
        );
    }
    if stats.backend_enabled_v4 {
        println!("Backend DHCP enabled (v4): yes");
    }
    if stats.backend_enabled_v6 {
        println!("Backend DHCP enabled (v6): yes");
    }
}

fn print_convert_stats(stats: &MigrationStats, backend: &Backend) {
    println!(
        "ISC DHCP static mappings found: {}",
        stats.isc_mappings_found
    );
    println!(
        "ISC DHCPv6 static mappings found: {}",
        stats.isc_mappings_v6_found
    );
    println!(
        "{} subnet4 entries found: {}",
        backend, stats.target_subnets_found
    );
    println!(
        "{} subnet6 entries found: {}",
        backend, stats.target_subnets_v6_found
    );
    println!("Reservations created: {}", stats.reservations_to_create);
    println!(
        "Reservations created (v6): {}",
        stats.reservations_v6_to_create
    );
    println!(
        "Reservations skipped (already exist): {}",
        stats.reservations_skipped
    );
    println!(
        "Reservations skipped (v6): {}",
        stats.reservations_v6_skipped
    );
}
