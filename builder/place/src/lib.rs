use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

mod db;
mod process;
mod target;

pub async fn process(
    input: &PathBuf,
    output: &Option<PathBuf>,
    test: bool,
    rename_with_ymd: bool,
) -> Result<()> {
    let output = if let Some(o) = output {
        o
    } else {
        &input.with_extension("mmfplace")
    };
    if !output.is_dir() {
        std::fs::create_dir_all(&output)?;
    }
    let (input, output) = (input.canonicalize()?, output.canonicalize()?);

    let total = walkdir::WalkDir::new(&input)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .count();
    info!(input=?input, total=total, output=?output, test=test, "start process");
    // init temp data
    process::temp_init(input, output, test, rename_with_ymd);
    process::do_process().await
}
