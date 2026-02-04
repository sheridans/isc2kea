use std::process;

fn main() {
    if let Err(e) = isc2kea::cli::run_with_args(std::env::args_os()) {
        eprintln!("Error: {:#}", e);
        process::exit(1);
    }
}
