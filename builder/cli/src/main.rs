use clap::{Parser, Subcommand, ValueHint};
use std::path::PathBuf;

use utils::log::setup_tracing;

#[derive(Subcommand, Debug)]
enum Commands {
    /// place files into directories by datetime
    Place {
        /// input file/directory path
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        input: PathBuf,
        /// test mode, do not copy/move file
        #[arg(long, default_value = "false")]
        test: bool,
    },
    /// find duplicate files
    Dupf {
        /// input file/directory path
        #[arg(short, long, value_hint = ValueHint::FilePath)]
        input: PathBuf,
    },
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(
    author = "idhyt",
    version = "dirty (81242966c1 2024-05-20)",
    about = "split multi-media file by earliest datetime",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// output directory path
    #[arg(short, long, global=true, value_hint = ValueHint::FilePath)]
    output: Option<PathBuf>,
    /// enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
    /// option point to the logfile path, must have RW permissions.
    #[arg(short, long, global=true, value_hint = ValueHint::FilePath)]
    logfile: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    // env_logger::init_from_env(Env::default().filter_or("LOG_LEVEL", "info"));
    // env_logger::init();
    let args = Cli::parse();
    setup_tracing(args.verbose, &args.logfile).expect("Failed to setup tracing");
    log::debug!("args: {:#?}", args);

    match &args.command {
        Commands::Place { input, test } => {
            if let Err(e) = place::process(input, &args.output, *test).await {
                log::error!("process error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Dupf { input } => {
            dupf::process(input, &args.output);
        }
    };
    std::process::exit(0);
}
