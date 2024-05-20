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
    /// point to the run directory, must have RW permissions
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    work_dir: Option<PathBuf>,
    /// input file/directory path
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    input: PathBuf,
    /// output directory path
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    output: Option<PathBuf>,
    /// custom config file path
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    config: Option<PathBuf>,
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

    setup_config(args.work_dir, args.config, args.output).await;

    match do_place(&args.input, args.test).await {
        Ok(_) => (),
        Err(e) => {
            log::error!("process error: {}", e);
            std::process::exit(1);
        }
    }
    std::process::exit(0);
}

async fn setup_config(
    work_dir: Option<PathBuf>,
    config_file: Option<PathBuf>,
    output: Option<PathBuf>,
) {
    use config::CONFIG;

    let mut config = CONFIG.lock().await;
    config.set_work_dir(work_dir);
    config.set_output_dir(output);
    config.load(config_file);
    // log::debug!("config: {:#?}", config);
    drop(config);
}
