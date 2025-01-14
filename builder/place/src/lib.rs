use anyhow::Result;
use chrono::{Datelike, Timelike, Utc};
use std::path::PathBuf;
use std::sync::Arc;
use walkdir::WalkDir;

use check::Checker;
use config::CONFIG;
use target::Target;

mod check;
mod meta;
mod parse;
mod target;

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

pub async fn do_place(input: &PathBuf, output: &PathBuf, test: bool) -> Result<()> {
    let total = get_total_size(input);

    log::info!(
        "start process with:\n  input: {:?}\n  output: {:?}\n  test: {}\n  total: {}",
        input,
        output,
        test,
        total
    );

    let mut index = 0;
    let mut handles = Vec::new();
    // let aout = Arc::new(output.to_path_buf());

    for entry in WalkDir::new(input) {
        let path = entry?.path().to_path_buf();
        let checker = Checker::new(&path);
        if checker.is_skip() {
            log::info!("skip file: {:?}", path);
            continue;
        }
        index += 1;

        handles.push(tokio::spawn(async move {
            Target::new(&path)
                //.process(index, total, Arc::clone(&atout))
                .process(index, total, None)
                .await
        }));

        if handles.len() >= CONFIG.batch_size {
            for handle in handles.iter_mut() {
                let target = handle.await??;
                target.copy(output);
            }
            handles.clear();
        }
    }

    // wait for all handles done
    if handles.len() > 0 {
        for handle in handles {
            let target = handle.await??;
            target.copy(output);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_total_size() {
        let path = PathBuf::from("/tmp/123");
        let total = get_total_size(&path);
        println!("total: {}", total);
        // assert_eq!(total, 3);
    }
}
