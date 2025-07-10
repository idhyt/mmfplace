use anyhow::Result;
use chrono::prelude::*;
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};

use utils::crypto::get_file_md5;

// output generation
pub static OUTPUT_GEN: Lazy<fn(&Path, &Vec<String>) -> PathBuf> = Lazy::new(|| {
    |o, p| {
        p.iter().fold(o.to_owned(), |mut path, p| {
            path.push(p);
            path
        })
    }
});

#[derive(Debug, Clone, Default)]
pub struct TimeInfo {
    // parsed datetime from metadata
    // all datetime parsed as to utc
    parsedtimes: Vec<DateTime<Utc>>,
    // datetime from file attributes
    // [accessed, modified, created]
    attrtimes: Vec<Option<SystemTime>>,
    // the earliest datetime, minimum of parsedtimes and attrtimes
    // set it to private `Option` system local time ensure every process should set it.
    earliest: Option<DateTime<Local>>,
}

// impl Default for TimeInfo {
//     fn default() -> Self {
//         TimeInfo {
//             parsedtimes: Vec::new(),
//             attrtimes: Vec::new(),
//             earliest: Some(DateTime::<Local>::from(SystemTime::now())),
//         }
//     }
// }

#[derive(Debug, Clone, Default)]
pub struct Target {
    // target file path
    pub path: PathBuf,
    // the path parts of the target placed
    parts: Option<Vec<String>>,
    // // parsed datetime from metadata
    // pub datetimes: Vec<DateTime<Utc>>,
    // hash with md5
    pub hash: String,
    // the original file
    pub extension: String,
    // the file name without extension
    pub name: String,
    // the file parsed type
    pub ftype: Option<String>,
    // the target file times info
    pub tinfo: TimeInfo,
    // // the earliest datetime
    // pub earliest: DateTime<Utc>,
    // // datetime from file attributes
    // // [accessed, modified, created]
    // pub attrtimes: Vec<Option<SystemTime>>,
    // // whether the file has been dealt with before
    pub dealt: bool,
    // the output path
    pub output: PathBuf,
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

    pub fn set_parts(&mut self, parts: Option<Vec<String>>) {
        if let Some(old) = &self.parts {
            if let Some(new) = &parts {
                warn!(file=?self.path, from = ?old, to = ?new, "⚠️ the parts will be overwrited");
            } else {
                warn!(file=?self.path, from = ?old, to = "None", "⚠️ the parts will be overwrited to empty");
            }
        } else {
            // debug!(file=?self.path, "set parts");
        }

        self.parts = parts;
    }

    pub fn get_parts(&self) -> Result<&[String]> {
        if self.parts.is_none() {
            return Err(anyhow::anyhow!("parts not set"));
        }
        Ok(&self.parts.as_ref().unwrap())
    }

    // 重名文件添加序号，是/否重命名文件
    pub fn get_name(&self, i: usize, rename: Option<&str>) -> String {
        let name = if let Some(n) = rename { n } else { &self.name };
        if i == 0 {
            format!(
                "{}.{}",
                name,
                self.ftype.as_ref().map_or(&self.extension, |s| &s)
            )
        } else {
            format!(
                "{}_{:02}.{}",
                name,
                i,
                self.ftype.as_ref().map_or(&self.extension, |s| &s)
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

    #[allow(dead_code)]
    pub fn get_attrtime(&self) -> &[Option<SystemTime>] {
        &self.tinfo.attrtimes
    }

    pub fn add_parsedtime(&mut self, dt: DateTime<Utc>) {
        self.tinfo.parsedtimes.push(dt);
    }

    #[allow(dead_code)]
    pub fn get_parsedtime(&self) -> &[DateTime<Utc>] {
        &self.tinfo.parsedtimes
    }

    pub fn set_earliest(&mut self, timestamp: Option<u64>) -> Result<()> {
        if let Some(ts) = timestamp {
            self.set_earliest_from_timestamp(ts);
        } else {
            self.update_earliest()?;
        }
        Ok(())
    }

    fn set_earliest_from_timestamp(&mut self, timestamp: u64) {
        let systime: SystemTime = UNIX_EPOCH + Duration::from_secs(timestamp);
        self.tinfo.earliest = Some(systime.into());
    }

    fn update_earliest(&mut self) -> Result<()> {
        // 最少包含 mtime 和 atime
        let attr_min = DateTime::<Local>::from(
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
            self.tinfo.earliest = Some(attr_min);
            warn!(file=?self.path, "💡 time not found by dateparser, use the attrtimes as earliest time");
        } else {
            // self.tinfo.earliest = self
            //     .tinfo
            //     .parsedtimes
            //     .iter()
            //     .fold(attr_min, |m, dt| *dt.min(&m));
            let parsed_min = self
                .tinfo
                .parsedtimes
                .iter()
                .min()
                .ok_or(anyhow::anyhow!("min time not found in parsedtimes"))?
                .with_timezone(&Local);
            self.tinfo.earliest = Some(parsed_min.min(attr_min));
            debug!(file=?self.path, "use the minimum time of attrtimes and dateparser");
        }
        info!(file=?self.path, earliest = ?self.tinfo.earliest, "🎉 success set earliest datetime");
        Ok(())
    }

    pub fn get_earliest(&self) -> Result<DateTime<Local>> {
        // 强制验证 earliest 是否设置过，否则说明逻辑处理存在缺陷
        self.tinfo
            .earliest
            .ok_or(anyhow::anyhow!("earliest time not set"))?;
        Ok(self.tinfo.earliest.unwrap())
    }

    pub fn set_output(&mut self, dir: &Path, rename_with_ymd: bool) -> Result<()> {
        // parse阶段标记：之前处理过了，会设置parts字段，直接返回路径
        if self.dealt {
            self.output = OUTPUT_GEN(dir, self.parts.as_ref().unwrap());
            return Ok(());
        }

        let earliest = self.get_earliest()?;
        // parse阶段没有标记，说明之前没处理过，生成新路径，并设置新的parts
        let mut parts: Vec<String> = vec![
            earliest.year().to_string(),
            // self.tinfo.earliest.month().to_string(),
            format!("{:02}", earliest.month()),
            "".to_string(),
        ];
        // 保留文件名格式
        let name: Option<String> = {
            if rename_with_ymd {
                Some(format!(
                    "{}-{:02}-{:02}",
                    earliest.year(),
                    earliest.month(),
                    earliest.day()
                ))
            } else {
                None
            }
        };
        // 有可能文件重名，循环生成
        for i in 0..1000 {
            parts[2] = self.get_name(i, name.as_deref());
            let check = OUTPUT_GEN(dir, &parts);
            // 文件不存在，表明该路径可用
            if !check.is_file() {
                self.output = check;
                // 更新 parts
                self.parts = Some(parts);
                return Ok(());
            }
            debug!(
                file = ?check,
                count = i+1,
                "already exist"
            );
        }

        // 如果循环1000次文件都存在，一定是有问题(存在大量的相同文件名且hash不同)
        error!(file=?self.path, "output generate too many tries");
        Err(anyhow::anyhow!(
            "output generate too many tries, file={}",
            self.path.display()
        ))
    }

    pub fn copy_with_times(&self) -> Result<()> {
        let output = &self.output;
        // 判断是否需要拷贝
        let need_copy = {
            if output.is_file() {
                // 文件存在且hash相同，则跳过
                if self.hash == get_file_md5(&output)? {
                    info!(file=?output, "🚚 copy skip with same hash");
                    false
                }
                // 文件存在，hash不同，被修改过，则直接覆盖
                else {
                    warn!(file=?output, "🚚 copy overwrite with different hash");
                    true
                }
            }
            // 文件不存在，两种情况：
            // 1. 正常的逻辑流程，之前没处理过
            // 2. 之前被处理过，但是被删除了
            else {
                info!(file=?output, "🚚 copy with file not exist");
                true
            }
        };
        // 不需要拷贝文件，直接返回
        if !need_copy {
            return Ok(());
        }

        let earliest = self.get_earliest()?;

        // 创建文件夹并赋值
        let dir = output.parent().ok_or(anyhow::anyhow!(
            "the output parent directory not found {:?}",
            &output
        ))?;
        if !dir.is_dir() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::copy(&self.path, output)?;

        // 设置拷贝文件的属性到最早时间
        let st: SystemTime = earliest.into();
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::fs::FileTimesExt;
            std::fs::File::options()
                .write(true)
                .open(output)?
                .set_times(
                    std::fs::FileTimes::new()
                        .set_accessed(st)
                        .set_modified(st)
                        .set_created(st),
                )?
        }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            std::fs::File::options()
                .write(true)
                .open(output)?
                .set_times(std::fs::FileTimes::new().set_accessed(st).set_modified(st))?
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

    #[test]
    fn test_output_gen() {
        let path = get_root().join("tests");
        let output = OUTPUT_GEN(
            path.as_path(),
            &vec![
                "2025".to_string(),
                "07".to_string(),
                "小鸡动画.gif".to_string(),
            ],
        );
        println!("general output: {:?}", output);
        assert_eq!(output, path.join("2025").join("07").join("小鸡动画.gif"));
    }
}
