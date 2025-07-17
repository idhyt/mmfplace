use std::path::PathBuf;
use tracing::Level;
use tracing_subscriber::{filter::Targets, prelude::*};

pub fn setup_tracing(
    verbose: bool,
    logfile: &Option<PathBuf>,
) -> Result<(), tracing_subscriber::util::TryInitError> {
    let filter = if verbose {
        Targets::new()
            // .with_target("ignore", Level::INFO)
            .with_default(Level::DEBUG)
    } else {
        Targets::default().with_default(Level::INFO)
    };
    let stdout_log = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_target(verbose)
        .with_line_number(verbose);
    let file_log = match logfile {
        Some(path) => {
            let file = std::fs::File::create(&path).expect("Failed to open logfile");
            // Some(tracing_subscriber::fmt::layer().json().with_writer(file))
            Some(tracing_subscriber::fmt::layer().with_writer(file))
        }
        None => None,
    };

    tracing_subscriber::registry()
        .with(stdout_log)
        .with(file_log)
        .with(filter)
        .try_init()
}
