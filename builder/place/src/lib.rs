use anyhow::Result;
use std::path::PathBuf;

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
    // init temp data
    process::do_process(input, output, test, rename_with_ymd).await
}
