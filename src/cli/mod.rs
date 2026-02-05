use anyhow::Result;
use clap::{Parser, Subcommand};
use std::ffi::OsString;

use crate::{Backend, MigrationStats};

mod convert;
mod scan;
mod verify;

pub(crate) struct ScanArgs {
    pub(crate) r#in: std::path::PathBuf,
    pub(crate) backend: Backend,
    pub(crate) fail_if_existing: bool,
    pub(crate) create_subnets: bool,
    pub(crate) force_subnets: bool,
    pub(crate) create_options: bool,
    pub(crate) force_options: bool,
    pub(crate) enable_backend: bool,
    pub(crate) verbose: bool,
}

pub(crate) struct ConvertArgs {
    pub(crate) r#in: std::path::PathBuf,
    pub(crate) backend: Backend,
    pub(crate) out: std::path::PathBuf,
    pub(crate) fail_if_existing: bool,
    pub(crate) create_subnets: bool,
    pub(crate) force_subnets: bool,
    pub(crate) create_options: bool,
    pub(crate) force_options: bool,
    pub(crate) enable_backend: bool,
    pub(crate) verbose: bool,
    pub(crate) force: bool,
}

pub(crate) struct VerifyArgs {
    pub(crate) r#in: std::path::PathBuf,
    pub(crate) backend: Backend,
    pub(crate) fail_if_existing: bool,
    pub(crate) create_subnets: bool,
    pub(crate) force_subnets: bool,
    pub(crate) create_options: bool,
    pub(crate) force_options: bool,
    pub(crate) enable_backend: bool,
    pub(crate) verbose: bool,
    pub(crate) quiet: bool,
}

#[derive(Parser)]
#[command(
    name = "isc2kea",
    about = "Migrate ISC DHCP static mappings to Kea/dnsmasq DHCP configurations",
    long_about = "Designed for OPNsense config.xml but may work with similar XML schemas.",
    after_help = "Examples:\n  isc2kea scan --in ./config.xml --create-subnets --create-options\n  isc2kea convert --in ./config.xml --out ./config.xml.new --create-subnets --create-options\n  isc2kea convert --in ./config.xml --out ./config.xml.new --backend dnsmasq --create-subnets --create-options\n\nRun 'isc2kea scan --help' or 'isc2kea convert --help' to see all flags."
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
        r#in: std::path::PathBuf,

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
        r#in: std::path::PathBuf,

        /// Target DHCP backend
        #[arg(short, long, value_enum, default_value_t = Backend::Kea)]
        backend: Backend,

        /// Output file path for converted XML
        #[arg(short, long)]
        out: std::path::PathBuf,

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

    /// Verify the migration by showing a diff (no files written)
    Verify {
        /// Input config.xml file path
        #[arg(short, long, default_value = "/conf/config.xml")]
        r#in: std::path::PathBuf,

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

        /// Suppress diff output (exit code still indicates changes)
        #[arg(long)]
        quiet: bool,
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
        } => scan::run_scan(ScanArgs {
            r#in,
            backend,
            fail_if_existing,
            create_subnets,
            force_subnets,
            create_options,
            force_options,
            enable_backend,
            verbose,
        }),
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
        } => convert::run_convert(ConvertArgs {
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
        }),
        Commands::Verify {
            r#in,
            backend,
            fail_if_existing,
            create_subnets,
            force_subnets,
            create_options,
            force_options,
            enable_backend,
            verbose,
            quiet,
        } => verify::run_verify(VerifyArgs {
            r#in,
            backend,
            fail_if_existing,
            create_subnets,
            force_subnets,
            create_options,
            force_options,
            enable_backend,
            verbose,
            quiet,
        }),
    }
}

pub(crate) fn print_scan_stats(stats: &MigrationStats, backend: &Backend) {
    println!(
        "ISC DHCP static mappings found: {}",
        stats.isc_mappings_found
    );
    println!(
        "ISC DHCPv6 static mappings found: {}",
        stats.isc_mappings_v6_found
    );
    println!("ISC DHCP ranges found: {}", stats.isc_ranges_found);
    println!("ISC DHCPv6 ranges found: {}", stats.isc_ranges_v6_found);
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

pub(crate) fn print_convert_stats(stats: &MigrationStats, backend: &Backend) {
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
