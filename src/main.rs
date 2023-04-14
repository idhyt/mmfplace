use clap::{Parser, ValueEnum, ValueHint};
use flexi_logger;
use std::path::PathBuf;

mod config;
mod place;

#[derive(ValueEnum, Clone, Debug, PartialEq)]
enum ModeType {
    /// test mode, do not copy/move file
    Test,
    /// Copy file to output directory
    Copy,
    /// Move file to output directory
    Move,
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(
    author = "idhyt",
    version = "0.1",
    about = "split multi-media file by earliest datetime",
    long_about = None
)]
struct Args {
    /// which mode to used
    #[arg(value_enum, default_value_t=ModeType::Copy)]
    mode: ModeType,
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
}

#[tokio::main]
async fn main() {
    // env_logger::init_from_env(Env::default().filter_or("LOG_LEVEL", "info"));
    // env_logger::init();
    let args = Args::parse();
    log::debug!("args: {:#?}", args);

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

    let mut config = config::Config::new();
    config = match args.config {
        Some(p) => config.update(p),
        None => config,
    };
    log::debug!("config: {:#?}", config);

    let work_dir = match args.work_dir {
        Some(wd) => wd,
        None => {
            let mut exe_dir = std::env::current_exe().expect("failed to get current exe path");
            exe_dir.pop();
            exe_dir
        }
    };

    let output = match args.output {
        Some(o) => o,
        None => work_dir.clone().join("output"),
    };
    if !output.is_dir() {
        std::fs::create_dir_all(&output).expect("failed to create output directory");
    }

    match place::process(
        work_dir,
        args.input,
        output,
        config,
        args.mode == ModeType::Test,
    )
    .await
    {
        Ok(_) => (),
        Err(e) => {
            log::error!("process error: {}", e);
            std::process::exit(1);
        }
    }
    std::process::exit(0);
}
