use anyhow::Result;
use chrono::prelude::*;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::{debug, error, info, warn};

use utils::crypto::get_file_md5;

#[derive(Debug, Clone)]
pub struct TimeInfo {
    // parsed datetime from metadata
    pub parsedtimes: Vec<DateTime<Utc>>,
    // datetime from file attributes
    // [accessed, modified, created]
    pub attrtimes: Vec<Option<SystemTime>>,
    // the earliest datetime, minimum of parsedtimes and attrtimes
    pub earliest: DateTime<Utc>,
}

impl Default for TimeInfo {
    fn default() -> Self {
        let now = SystemTime::now();
        TimeInfo {
            parsedtimes: Vec::new(),
            attrtimes: Vec::new(),
            earliest: DateTime::<Utc>::from(now),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Target {
    // target file path
    pub path: PathBuf,
    // the path parts of the target placed
    pub parts: Option<Vec<String>>,
    // // parsed datetime from metadata
    // pub datetimes: Vec<DateTime<Utc>>,
    // hash with md5
    pub hash: String,
    // the original file
    pub extension: String,
    // the file name without extension
    pub name: String,
    // the file parsed type
    pub type_: Option<String>,
    // the target file times info
    pub tinfo: TimeInfo,
    // // the earliest datetime
    // pub earliest: DateTime<Utc>,
    // // datetime from file attributes
    // // [accessed, modified, created]
    // pub attrtimes: Vec<Option<SystemTime>>,
    // // whether the file has been dealt with before
    pub dealt: bool,
}

impl Target {
    pub fn new(path: PathBuf) -> Result<Self> {
        let mut target = Target {
            hash: get_file_md5(&path)?,
            extension: path
                .extension()
                .map_or("bin".to_string(), |e| e.to_string_lossy().to_lowercase()),
            name: path
                .file_stem()
                .map_or("NoName".to_string(), |n| n.to_string_lossy().to_lowercase()),
            path,
            ..Default::default()
        };
        target.set_attrtimes()?;
        Ok(target)
    }

    // 重名文件添加序号，是/否重命名文件
    pub fn get_name(&self, i: usize, rename: Option<&str>) -> String {
        let name = if let Some(n) = rename { n } else { &self.name };
        if i == 0 {
            format!(
                "{}.{}",
                name,
                self.type_.as_ref().map_or(&self.extension, |s| &s)
            )
        } else {
            format!(
                "{}_{:02}.{}",
                name,
                i,
                self.type_.as_ref().map_or(&self.extension, |s| &s)
            )
        }
    }

    fn set_attrtimes(&mut self) -> Result<()> {
        let meta = std::fs::metadata(&self.path)?;
        // if let Ok(atime) = meta.accessed() {
        //     self.tinfo.attrtimes.push(Some(atime));
        // } else {
        //     warn!(file=?self.path, "💡 accessed time not found");
        //     self.tinfo.attrtimes.push(None);
        // }
        // if let Ok(mtime) = meta.modified() {
        //     self.tinfo.attrtimes.push(Some(mtime));
        // } else {
        //     warn!(file=?self.path, "💡 modified time not found");
        //     self.tinfo.attrtimes.push(None);
        // }
        self.tinfo.attrtimes.push(Some(meta.accessed()?));
        self.tinfo.attrtimes.push(Some(meta.modified()?));
        // #[cfg(windows)] only support in Windows
        if let Ok(ctime) = meta.created() {
            self.tinfo.attrtimes.push(Some(ctime));
        } else {
            debug!(file=?self.path, "💡 created time not found(Non-Windows?)");
            self.tinfo.attrtimes.push(None);
        }
        Ok(())
    }

    pub fn set_earliest(&mut self) -> Result<()> {
        // 最少包含 mtime 和 atime
        let attr_min = DateTime::<Utc>::from(
            *self
                .tinfo
                .attrtimes
                .iter()
                .flatten()
                .min()
                .ok_or(anyhow::anyhow!("min time not found in attrtimes"))?,
        );

        if self.tinfo.parsedtimes.is_empty() {
            // should panic?
            // warn!(file=?self.path, "💡 datetime not found by dateparser");
            self.tinfo.earliest = attr_min;
            debug!(file=?self.path, "💡 time not found by dateparser, use the attrtimes as earliest time");
        } else {
            self.tinfo.earliest = self
                .tinfo
                .parsedtimes
                .iter()
                .fold(attr_min, |m, dt| *dt.min(&m));
            debug!(file=?self.path, "use the minimum time of attrtimes and dateparser");
        }
        info!(file=?self.path, earliest = ?self.tinfo.earliest, "🎉 success set earliest datetime");
        Ok(())
    }

    pub fn get_output(&mut self, dir: &Path, rename_with_ymd: bool) -> Result<Option<PathBuf>> {
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
                    self.tinfo.earliest.year().to_string(),
                    // self.tinfo.earliest.month().to_string(),
                    format!("{:02}", self.tinfo.earliest.month()),
                    "".to_string(),
                ];
                let name: Option<String> = {
                    if rename_with_ymd {
                        Some(format!(
                            "{}-{:02}-{:02}",
                            self.tinfo.earliest.year(),
                            self.tinfo.earliest.month(),
                            self.tinfo.earliest.day()
                        ))
                    } else {
                        None
                    }
                };

                let output = (0..1000).find_map(|i| {
                    parts[2] = self.get_name(i, name.as_deref());
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

    pub fn copy_file_with_times(&self, dst: &Path) -> Result<()> {
        let dir = dst.parent().ok_or(anyhow::anyhow!(
            "the output parent directory not found {:?}",
            &dst
        ))?;
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::copy(&self.path, dst)?;

        // we set the all time to min
        let st: SystemTime = self.tinfo.earliest.into();
        if cfg!(target_os = "windows") {
            use std::os::windows::fs::FileTimesExt;
            std::fs::File::options().write(true).open(dst)?.set_times(
                std::fs::FileTimes::new()
                    .set_accessed(st)
                    .set_modified(st)
                    .set_created(st),
            )?;
        } else {
            std::fs::File::options()
                .write(true)
                .open(dst)?
                .set_times(std::fs::FileTimes::new().set_accessed(st).set_modified(st))?;
        }
        Ok(())
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
        let target = Target::new(path).unwrap();
        println!("target: {:#?}", target);
        assert_eq!(target.hash, "a6cc791ccd13f0dea507b0eb0f2c1b47");
        assert_eq!(target.extension, "gif");
        assert_eq!(target.name, "小鸡动画");
    }

    #[test]
    fn test_get_name() {
        let path = get_root().join("tests").join("2025/07/小鸡动画.gif");
        let target = Target::new(path).unwrap();
        assert_eq!(target.get_name(1, Some("abc")), "abc_01.gif");
    }
}
