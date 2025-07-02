use anyhow::Result;
use chrono::{Datelike, Timelike, Utc};
use std::path::PathBuf;
// use std::sync::Arc;
use tracing::{debug, debug_span, info};
use tracing_futures::Instrument;
use walkdir::WalkDir;

use check::Checker;
use config::CONFIG;
use target::Target;

pub mod check;
mod parse;
mod target;

static mut ISTEST: bool = false;

pub fn panic_with_test() {
    if unsafe { ISTEST } {
        panic!("-------- panic in testing mode, try to run with -v to see the detail --------");
    }
}

#[derive(Debug, Clone, Default)]
pub struct FileDateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub timestamp: i64,
}

impl FileDateTime {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            year: now.year() as u16,
            month: now.month() as u8,
            day: now.day() as u8,
            hour: now.hour() as u8,
            minute: now.minute() as u8,
            second: now.second() as u8,
            timestamp: now.timestamp() as i64,
        }
    }
}

// impl display for FileDateTime
impl std::fmt::Display for FileDateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:04}:{:02}:{:02} {:02}:{:02}:{:02}, {}",
            self.year, self.month, self.day, self.hour, self.minute, self.second, self.timestamp
        )
    }
}

fn get_total_size(path: &PathBuf) -> usize {
    WalkDir::new(path)
        .into_iter()
        .filter(|e| {
            let p = e.as_ref().unwrap().path().to_path_buf();
            !Checker::new(&p).is_skip()
        })
        .count()
}

pub async fn process(input: &PathBuf, output: &Option<PathBuf>, test: bool) -> Result<()> {
    unsafe {
        ISTEST = test;
    }
    let output = if let Some(o) = output {
        o.to_path_buf()
    } else {
        PathBuf::from(format!("{}.mmfplace", input.to_str().unwrap()))
    };

    let total = get_total_size(input);

    info!(
        "start process with:\n  input: {:?}\n  output: {:?}\n  test: {}\n  total: {}",
        input, output, test, total
    );

    let mut index = 0;
    let mut handles = Vec::new();
    // let aout = Arc::new(output.to_path_buf());

    let root_span = debug_span!("process");
    let _enter = root_span.enter();

    for entry in WalkDir::new(input) {
        let path = entry?.path().to_path_buf();
        let checker = Checker::new(&path);
        if checker.is_skip() {
            debug!("skip file: {:?}", path);
            continue;
        }
        index += 1;

        handles.push(tokio::spawn(
            async move {
                let span = debug_span!("async_task", path = ?path, index = index, total = total);
                async {
                    Target::new(&path, index, total)
                        //.process(index, total, Arc::clone(&atout))
                        .process(None)
                        .await
                }
                .instrument(span)
                .await
            }
            .instrument(root_span.clone()),
        ));

        if handles.len() >= CONFIG.batch_size {
            for handle in handles.iter_mut() {
                let target = handle.await??;
                target.copy(&output);
            }
            handles.clear();
        }
    }

    // wait for all handles done
    if handles.len() > 0 {
        for handle in handles {
            let target = handle.await??;
            target.copy(&output);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_root() -> PathBuf {
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }
    #[test]
    fn test_get_total_size() {
        let path = get_root().join("tests");
        let total = get_total_size(&path);
        println!("total: {}", total);
        // assert_eq!(total, 3);
    }
}
