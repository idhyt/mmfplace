use anyhow::Result;
use chrono::{Datelike, Timelike, Utc};
use std::path::PathBuf;

pub mod entry;
pub mod meta;
pub mod pick;

#[derive(Debug, Clone)]
pub struct FileDateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub file_path: PathBuf,
    pub suffix: String,
    pub datetime: FileDateTime,
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

    pub fn to_string(&self) -> String {
        format!(
            "{:04}:{:02}:{:02} {:02}:{:02}:{:02}, {}",
            self.year, self.month, self.day, self.hour, self.minute, self.second, self.timestamp
        )
    }
}

impl FileInfo {
    pub fn new(file_path: &PathBuf) -> Self {
        Self {
            file_path: file_path.to_path_buf(),
            suffix: "".to_string(),
            datetime: FileDateTime::new(),
        }
    }

    pub fn get_name(&self, index: u32) -> String {
        if index == 0 {
            return format!(
                "{:04}-{:02}-{:02}-{:02}-{:02}-{:02}.{}",
                self.datetime.year,
                self.datetime.month,
                self.datetime.day,
                self.datetime.hour,
                self.datetime.minute,
                self.datetime.second,
                self.suffix
            );
        } else {
            return format!(
                "{:04}-{:02}-{:02}-{:02}-{:02}-{:02}_{:05}.{}",
                self.datetime.year,
                self.datetime.month,
                self.datetime.day,
                self.datetime.hour,
                self.datetime.minute,
                self.datetime.second,
                index,
                self.suffix
            );
        }
    }
}

pub async fn do_place(input: &PathBuf, test: bool) -> Result<()> {
    entry::process(input, test).await
}
