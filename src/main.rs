use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::fs::{File, OpenOptions};
use std::io::Cursor;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process;

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
        #[arg(short, long, value_enum, default_value_t = isc2kea::Backend::Kea)]
        backend: isc2kea::Backend,

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
        #[arg(short, long, value_enum, default_value_t = isc2kea::Backend::Kea)]
        backend: isc2kea::Backend,

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

        /// Show detailed progress for each mapping
        #[arg(short, long)]
        verbose: bool,

        /// Overwrite output file if it exists
        #[arg(long)]
        force: bool,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan {
            r#in,
            backend,
            fail_if_existing,
            create_subnets,
            force_subnets,
            create_options,
            force_options,
            verbose,
        } => {
            let mut file = File::open(&r#in)
                .with_context(|| format!("Failed to open input file: {}", r#in.display()))?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .with_context(|| format!("Failed to read input file: {}", r#in.display()))?;

            let options = isc2kea::MigrationOptions {
                fail_if_existing,
                verbose,
                backend: backend.clone(),
                create_subnets,
                force_subnets,
                create_options,
                force_options,
            };

            let stats = match isc2kea::scan_config(Cursor::new(&buffer), &options) {
                Ok(stats) => stats,
                Err(e) => {
                    if let Some(migration_error) = e.downcast_ref::<isc2kea::MigrationError>() {
                        if matches!(
                            migration_error,
                            isc2kea::MigrationError::BackendNotConfigured { .. }
                                | isc2kea::MigrationError::NoBackendSubnets { .. }
                                | isc2kea::MigrationError::BackendV6NotConfigured { .. }
                                | isc2kea::MigrationError::NoBackendSubnetsV6 { .. }
                        ) {
                            if let Ok(stats) = isc2kea::scan_counts(Cursor::new(&buffer), &backend)
                            {
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

            // Safer output creation: fail if exists unless --force
            let output_file = if force {
                OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&out)
                    .with_context(|| {
                        format!("Failed to open output file for writing: {}", out.display())
                    })?
            } else {
                match OpenOptions::new().write(true).create_new(true).open(&out) {
                    Ok(file) => file,
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                        bail!(
                            "Output file already exists: {} (use --force to overwrite)",
                            out.display()
                        );
                    }
                    Err(e) => {
                        return Err(e).with_context(|| {
                            format!("Failed to create output file: {}", out.display())
                        });
                    }
                }
            };

            let options = isc2kea::MigrationOptions {
                fail_if_existing,
                verbose,
                backend: backend.clone(),
                create_subnets,
                force_subnets,
                create_options,
                force_options,
            };

            let stats = isc2kea::convert_config(input_file, output_file, &options)?;

            println!("\nMigration completed successfully!");
            print_convert_stats(&stats, &backend);
            println!("Output written to: {}", out.display());
        }
    }

    Ok(())
}

fn print_scan_stats(stats: &isc2kea::MigrationStats, backend: &isc2kea::Backend) {
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
}

fn print_convert_stats(stats: &isc2kea::MigrationStats, backend: &isc2kea::Backend) {
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
