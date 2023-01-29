use clap::{Parser, ValueHint};
use flexi_logger;
use std::path::PathBuf;

use config::config;
use splits::SplitsProcess;

mod splits;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(
    author = "idhyt",
    version = "0.1",
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
    /// test mode, do not move file
    #[arg(long, default_value = "false")]
    test: bool,
}

async fn async_main(
    work_dir: Option<PathBuf>,
    input: PathBuf,
    output: Option<PathBuf>,
    config: config::Config,
    test: bool,
) {
    let splits = SplitsProcess::new(work_dir, input, output, config, test)
        .await
        .unwrap_or_else(|e| {
            log::error!("create splits process failed: {}", e);
            panic!();
        });
    match splits.run().await {
        Ok(_) => log::info!("splits process finished"),
        Err(e) => {
            log::error!("splits process failed: {}", e);
            panic!();
        }
    };
}

fn main() {
    // env_logger::init_from_env(Env::default().filter_or("LOG_LEVEL", "info"));
    // env_logger::init();
    let args = Args::parse();

    let mut logger = flexi_logger::Logger::try_with_env_or_str("info")
        .expect("Could not create Logger from environment :(");
    match &args.logfile {
        Some(p) => {
            let filespec = flexi_logger::FileSpec::try_from(p.to_str().unwrap())
                .expect("invalid logfile path");
            logger = logger
                .log_to_file(filespec)
                .write_mode(flexi_logger::WriteMode::Async)
                .duplicate_to_stdout(flexi_logger::Duplicate::All)
            // .start()
            // .expect("failed to initialize logger!");
        }
        None => (), // flexi_logger::FileSpec::default(),
    };
    let _logger = logger.start().expect("failed to initialize logger!");

    log::info!("args: {:#?}", args);

    let mut config = config::Config::new();

    config = match args.config {
        Some(p) => config.update(p),
        None => config,
    };

    futures::executor::block_on(async_main(
        args.work_dir,
        args.input,
        args.output,
        config,
        args.test,
    ));
}
