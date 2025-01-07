use chrono::{Datelike, Timelike, Utc};
use std::path::PathBuf;

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

#[derive(Debug, Clone)]
pub struct Target {
    pub path: PathBuf,
    pub suffix: String,
    pub datetime: FileDateTime,
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}, {}, {}",
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
            suffix: path
                .extension()
                .map_or_else(|| "".to_string(), |s| s.to_string_lossy().to_string()),
            datetime: FileDateTime::new(),
        }
    }

    // mark maybe the file hasher like Some("a18932e314dbb4c81c6fd0e282d81d16") or None
    pub fn get_name(&self, mark: Option<&str>) -> String {
        if let Some(mark) = mark {
            format!(
                "{:04}-{:02}-{:02}-{:02}-{:02}-{:02}_{}.{}",
                self.datetime.year,
                self.datetime.month,
                self.datetime.day,
                self.datetime.hour,
                self.datetime.minute,
                self.datetime.second,
                mark,
                self.suffix
            )
        } else {
            format!(
                "{:04}-{:02}-{:02}-{:02}-{:02}-{:02}.{}",
                self.datetime.year,
                self.datetime.month,
                self.datetime.day,
                self.datetime.hour,
                self.datetime.minute,
                self.datetime.second,
                self.suffix
            )
        }
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
        assert_eq!(target.suffix, "jpg");
    }

    #[test]
    fn test_target_get_name() {
        let path = PathBuf::from("/tmp/mmfplace-tests/simple.jpg");
        let target = Target::new(&path);
        let name = target.get_name(None);
        println!("target: {}, name: {}", target, name);
        assert!(name.contains(&format!("{}.jpg", target.datetime.second)));
        let name = target.get_name(Some("a18932e314dbb4c81c6fd0e282d81d16"));
        println!("target: {}, name: {}", target, name);
        assert!(name.contains("_a18932e314dbb4c81c6fd0e282d81d16.jpg"));
    }
}
