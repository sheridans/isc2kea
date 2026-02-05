use crate::{convert_config, MigrationOptions};
use anyhow::{bail, Context, Result};
use std::fs::{File, OpenOptions};
use std::io;

use super::print_convert_stats;
use super::ConvertArgs;

pub(crate) fn run_convert(args: ConvertArgs) -> Result<()> {
    // Critical safety check: prevent input == output
    let in_canonical = std::fs::canonicalize(&args.r#in).unwrap_or_else(|_| args.r#in.clone());
    let (out_canonical, out_missing) = match std::fs::canonicalize(&args.out) {
        Ok(path) => (path, false),
        Err(e) => (args.out.clone(), e.kind() == io::ErrorKind::NotFound),
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
        if let (Some(parent), Some(file_name)) = (args.out.parent(), args.out.file_name()) {
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

    let input_file = File::open(&args.r#in)
        .with_context(|| format!("Failed to open input file: {}", args.r#in.display()))?;

    if !args.force && args.out.exists() {
        bail!(
            "Output file already exists: {} (use --force to overwrite)",
            args.out.display()
        );
    }

    let tmp_path = args
        .out
        .with_extension(format!("tmp.{}", std::process::id()));
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
        fail_if_existing: args.fail_if_existing,
        verbose: args.verbose,
        backend: args.backend.clone(),
        create_subnets: args.create_subnets,
        force_subnets: args.force_subnets,
        create_options: args.create_options,
        force_options: args.force_options,
        enable_backend: args.enable_backend,
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

    if args.force && args.out.exists() {
        std::fs::remove_file(&args.out).with_context(|| {
            format!(
                "Failed to remove existing output file: {}",
                args.out.display()
            )
        })?;
    }

    std::fs::rename(&tmp_path, &args.out)
        .with_context(|| format!("Failed to replace output file: {}", args.out.display()))?;

    println!("\nMigration completed successfully!");
    print_convert_stats(&stats, &args.backend);
    println!("Output written to: {}", args.out.display());

    Ok(())
}
