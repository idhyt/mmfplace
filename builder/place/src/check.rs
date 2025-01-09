use anyhow::Result;
use std::path::PathBuf;

pub struct Checker<'a> {
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
