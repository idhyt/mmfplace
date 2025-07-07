use anyhow::Result;
use chrono::prelude::*;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::{debug, error, info, warn};

use utils::crypto::get_file_md5;

#[derive(Debug, Clone, Default)]
pub struct Target {
    // target file path
    pub path: PathBuf,
    // the path parts of the target placed
    pub parts: Option<Vec<String>>,
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
    // whether the file has been dealt with before
    pub dealt: bool,
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

    pub fn get_output(&mut self, dir: &Path) -> Result<Option<PathBuf>> {
        let generation = |o: &Path, p: &Vec<String>| {
            p.iter().fold(o.to_owned(), |mut path, p| {
                path.push(p);
                path
            })
        };

        let output = {
            // parse阶段标记：之前处理过了，会设置parts字段，直接返回路径
            if self.dealt {
                generation(dir, self.parts.as_ref().unwrap())
            }
            // parse阶段没有标记，说明之前没处理过，生成新路径，并设置新的parts
            // 有可能文件重名，循环生成
            else {
                let mut parts: Vec<String> = vec![
                    self.earliest.year().to_string(),
                    // self.earliest.month().to_string(),
                    format!("{:02}", self.earliest.month()),
                    "".to_string(),
                ];
                let output = (0..1000).find_map(|i| {
                    parts[2] = self.get_name(i);
                    let check = generation(dir, &parts);
                    // 文件不存在，表明该路径可用
                    if !check.is_file() {
                        Some(check)
                    } else {
                        debug!(
                            file = ?check,
                            count = i+1,
                            "already exist"
                        );
                        None
                    }
                });
                // 如果循环1000次文件都存在，一定是有问题(存在大量的相同文件名且hash不同)
                if output.is_none() {
                    error!(file=?self.path, "output generate too many tries");
                    return Err(anyhow::anyhow!(
                        "output generate too many tries, file={}",
                        self.path.display()
                    ));
                }
                // 更新 parts
                self.parts = Some(parts);
                output.unwrap()
            }
        };

        // 判断是否需要拷贝
        if output.is_file() {
            // 文件存在且hash相同，则跳过
            if self.hash == get_file_md5(&output).unwrap() {
                info!(file=?output, "🚚 copy skip with same hash");
                return Ok(None);
            }
            // 文件存在，hash不同，被修改过，则直接覆盖
            else {
                warn!(file=?output, "🚚 copy overwrite with different hash");
                return Ok(Some(output));
            }
        }
        // 文件不存在，两种情况：
        // 1. 正常的逻辑流程，之前没处理过
        // 2. 之前被处理过，但是被删除了
        else {
            info!(file=?output, "🚚 copy with file not exist");
            return Ok(Some(output));
        }
    }
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
    fn test_target() {
        let path = get_root().join("tests").join("2025/07/小鸡动画.gif");
        let target = Target::new(path);
        println!("target: {:#?}", target);
        assert_eq!(target.hash, "a6cc791ccd13f0dea507b0eb0f2c1b47");
        assert_eq!(target.extension, "gif");
        assert_eq!(target.name, "小鸡动画");
    }
}
