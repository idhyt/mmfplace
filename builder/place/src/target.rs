use anyhow::Result;
use chrono::{Datelike, Timelike, Utc};
use std::path::PathBuf;

use utils::crypto::get_file_md5;

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

#[derive(Debug, Clone, Default)]
pub struct Target {
    pub path: PathBuf,
    pub suffix: Option<String>,
    pub datetime: FileDateTime,
    pub hash: String,
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}, {:?}, {}",
            self.path.display(),
            self.suffix,
            self.datetime
        )
    }
}

impl Target {
    pub fn new(path: &PathBuf) -> Self {
        Self {
            path: path.to_path_buf(),
            // suffix: path
            //     .extension()
            //     .map_or_else(|| "bin".to_string(), |s| s.to_string_lossy().to_string()),
            suffix: path
                .extension()
                .map_or(None, |s| Some(s.to_string_lossy().to_string())),
            datetime: FileDateTime::new(),
            hash: get_file_md5(path).unwrap(),
        }
    }

    pub fn get_name(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}-{:02}-{:02}-{:02}.{}.{}",
            self.datetime.year,
            self.datetime.month,
            self.datetime.day,
            self.datetime.hour,
            self.datetime.minute,
            self.datetime.second,
            self.hash,
            self.suffix.as_ref().map_or("bin", |s| &s)
        )
    }

    // // mark maybe the file hasher like Some("a18932e314dbb4c81c6fd0e282d81d16") or None
    // pub fn get_name(&self, mark: Option<&str>) -> String {
    //     if let Some(mark) = mark {
    //         format!(
    //             "{:04}-{:02}-{:02}-{:02}-{:02}-{:02}_{}.{}",
    //             self.datetime.year,
    //             self.datetime.month,
    //             self.datetime.day,
    //             self.datetime.hour,
    //             self.datetime.minute,
    //             self.datetime.second,
    //             mark,
    //             self.suffix
    //         )
    //     } else {
    //         format!(
    //             "{:04}-{:02}-{:02}-{:02}-{:02}-{:02}.{}",
    //             self.datetime.year,
    //             self.datetime.month,
    //             self.datetime.day,
    //             self.datetime.hour,
    //             self.datetime.minute,
    //             self.datetime.second,
    //             self.suffix
    //         )
    //     }
    // }
}

struct Checker<'a> {
    pub path: &'a PathBuf,
    pub path_str: &'a str,
}

impl Checker<'_> {
    pub fn new<'a>(path: &'a PathBuf) -> Checker<'a> {
        Checker {
            path,
            path_str: path.to_str().unwrap(),
        }
    }

    // 非文件，或者已被处理过，将示为忽略
    pub fn is_ignore(&self) -> bool {
        if !self.path.is_file() {
            return true;
        }

        if self.path_str.to_lowercase().ends_with(".mmfplace") {
            return true;
        }

        false
    }

    // 当存在占位文件，则表示已处理过
    pub fn is_placed(&self) -> bool {
        let placed = PathBuf::from(format!("{}.mmfplace", self.path_str));
        placed.is_file()
    }

    // 处理过程中是否跳过，跳过条件：is_ignore || is_placed
    pub fn is_skip(&self) -> bool {
        if self.is_ignore() {
            // log::info!("skip ignore file: {}", self.path_str);
            return true;
        }
        if self.is_placed() {
            // log::info!("skip placed file: {}", self.path_str);
            return true;
        }
        false
    }

    // 设置占位文件
    pub fn set_placed(&self) -> Result<()> {
        let placed = PathBuf::from(format!("{}.mmfplace", self.path_str));
        std::fs::write(placed, "")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_new() {
        let path = PathBuf::from("/tmp/mmfplace-tests/simple.jpg");
        let target = Target::new(&path);
        println!("{}", target);
        assert_eq!(target.path, path);
        assert_eq!(target.suffix.unwrap(), "jpg");
    }

    #[test]
    fn test_target_get_name() {
        let path = PathBuf::from("/tmp/mmfplace-tests/simple.jpg");
        let target = Target::new(&path);
        let name = target.get_name();
        println!("target: {}, name: {}", target, name);
        assert!(name.contains(&format!("{}.{}.jpg", target.datetime.second, target.hash)));
        let name = target.get_name();
        println!("target: {}, name: {}", target, name);
        assert!(name.contains("a18932e314dbb4c81c6fd0e282d81d16.jpg"));
    }

    #[test]
    fn test_checker() {
        let path = PathBuf::from("/tmp/mmfplace-tests/simple.jpg");
        let placed = PathBuf::from("/tmp/mmfplace-tests/simple.jpg.mmfplace");
        if placed.is_file() {
            std::fs::remove_file(&placed).unwrap();
        }

        let checker = Checker::new(&path);
        assert!(!checker.is_ignore());
        assert!(!checker.is_placed());
        assert!(!checker.is_skip());
        checker.set_placed().unwrap();

        assert!(placed.is_file());
        assert!(checker.is_placed());
        assert!(checker.is_skip());

        std::fs::remove_file(placed).unwrap();
    }
}
