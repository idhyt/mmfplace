use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

mod db;
mod process;
mod target;

pub async fn process(input: &PathBuf, output: &Option<PathBuf>, test: bool) -> Result<()> {
    let intput = input.canonicalize()?;
    let output = if let Some(o) = output {
        o.canonicalize()?
    } else {
        intput.with_extension("mmfplace")
    };
    if !output.is_dir() && !test {
        std::fs::create_dir_all(&output)?;
    }
    let total = walkdir::WalkDir::new(&input)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .count();
    dbg!(total);
    info!(input=?input, total=total, output=?output, test=test, "start process");
    // init temp data
    process::temp_init(intput, output, test);
    process::do_process().await
}
