use chrono::prelude::*;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::time::SystemTime;
use tracing::{debug, error, info, warn};

use utils::crypto::get_file_md5;

#[derive(Debug, Clone, Default)]
pub struct Target {
    // target file path
    pub path: PathBuf,
    // parsed datetime from metadata
    pub datetimes: Vec<DateTime<Utc>>,
    // hash with md5
    pub hash: String,
    // the original file
    pub extension: String,
    // the file name without extension
    pub name: String,
    // the file parsed type
    pub type_: Option<String>,
    // the earliest datetime
    pub earliest: DateTime<Utc>,
    // datetime from file attributes
    // [accessed, modified, created]
    pub attrtimes: Vec<Option<SystemTime>>,
}

impl Target {
    pub fn new(path: PathBuf) -> Self {
        let mut target = Target {
            hash: get_file_md5(&path).unwrap(),
            extension: path
                .extension()
                .map_or("bin".to_string(), |e| e.to_string_lossy().to_lowercase()),
            name: path
                .file_stem()
                .map_or("NoName".to_string(), |n| n.to_string_lossy().to_lowercase()),
            path,
            ..Default::default()
        };
        target.set_attrtimes();
        target
    }

    // 重名文件添加序号
    pub fn get_name(&self, i: usize) -> String {
        if i == 0 {
            format!(
                "{}.{}",
                self.name,
                self.type_.as_ref().map_or(&self.extension, |s| &s)
            )
        } else {
            format!(
                "{}_{:02}.{}",
                self.name,
                i,
                self.type_.as_ref().map_or(&self.extension, |s| &s)
            )
        }
    }

    pub fn get_parts(&self, i: usize) -> Vec<String> {
        vec![
            self.earliest.year().to_string(),
            self.earliest.month().to_string(),
            self.earliest.day().to_string(),
            self.get_name(i),
        ]
    }

    pub fn add_datetime(&mut self, dt: DateTime<Utc>) {
        self.datetimes.push(dt);
    }

    pub fn set_attrtimes(&mut self) {
        let meta = std::fs::metadata(&self.path).unwrap();
        if let Ok(atime) = meta.accessed() {
            self.attrtimes.push(Some(atime));
        } else {
            warn!(file=?self.path, "💡 accessed time not found");
            self.attrtimes.push(None);
        }
        if let Ok(mtime) = meta.modified() {
            self.attrtimes.push(Some(mtime));
        } else {
            warn!(file=?self.path, "💡 modified time not found");
            self.attrtimes.push(None);
        }
        // #[cfg(windows)] only support in Windows
        if let Ok(ctime) = meta.created() {
            self.attrtimes.push(Some(ctime));
        } else {
            debug!(file=?self.path, "💡 created time not found(Non-Windows?)");
            self.attrtimes.push(None);
        }
    }

    pub fn set_earliest(&mut self) {
        if self.datetimes.is_empty() {
            // should panic?
            warn!(file=?self.path, "💡 datetime not found by dateparser")
        }
        let mut all = self
            .attrtimes
            .iter()
            .filter_map(|ost| ost.as_ref().map(|st| DateTime::<Utc>::from(*st)))
            .collect::<Vec<DateTime<Utc>>>();
        all.extend(self.datetimes.clone());

        if all.is_empty() {
            // self.earliest = Utc::now();
            // should panic
            error!(file=?self.path, "💥 datetime not found by dateparser and attributes!");
            panic!()
        }
        // min
        self.earliest = all.into_iter().min().unwrap();
        info!(file=?self.path, earliest = ?self.earliest, "🎉 success set earliest datetime");
    }
}
