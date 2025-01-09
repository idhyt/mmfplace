use clap::{Parser, ValueHint};
use std::path::PathBuf;

use place::do_place;
use utils::log::setup_tracing;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(
    author = "idhyt",
    version = "dirty (81242966c1 2024-05-20)",
    about = "split multi-media file by earliest datetime",
    long_about = None
)]
struct Args {
    /// input file/directory path
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    input: PathBuf,
    /// output directory path
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    output: PathBuf,
    /// custom the logfile path
    #[arg(long, value_hint = ValueHint::FilePath)]
    logfile: Option<PathBuf>,
    /// enable verbose logging
    #[arg(short, long, default_value = "false")]
    verbose: bool,
    /// test mode, do not copy/move file
    #[arg(long, default_value = "false")]
    test: bool,
}

#[tokio::main]
async fn main() {
    // env_logger::init_from_env(Env::default().filter_or("LOG_LEVEL", "info"));
    // env_logger::init();
    let args = Args::parse();

    setup_tracing(args.verbose, &args.logfile).expect("Failed to setup tracing");
    log::debug!("args: {:#?}", args);

    match do_place(&args.input, &args.output, args.test).await {
        Ok(_) => (),
        Err(e) => {
            log::error!("process error: {}", e);
            std::process::exit(1);
        }
    }
    std::process::exit(0);
}
