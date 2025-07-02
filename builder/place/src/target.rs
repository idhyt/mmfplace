use anyhow::Result;
// use filetime::{set_file_times, FileTime};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

use super::check::Checker;
use super::parse::{
    capture_type, get_datatime_from_metadata, get_datetime_from_additional,
    get_datetime_from_string, get_earliest_datetime_from_attributes,
};
use super::{panic_with_test, FileDateTime, ISTEST};

use config::CONFIG;
use tools::metadata::METADATA;
use utils::crypto::get_file_md5;

#[derive(Debug, Clone, Default)]
pub struct Target {
    pub index: usize,
    pub total: usize,
    /// target path
    pub path: PathBuf,
    /// the origin file extension with lower case
    pub extension: String,
    /// fixed file extension
    pub suffix: Option<String>,
    pub datetime: FileDateTime,
    /// file hash, md5 used now
    pub hash: String,
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}, {}, {}",
            self.path.display(),
            self.extension,
            self.datetime
        )
    }
}

impl Target {
    pub fn new(path: &PathBuf, index: usize, total: usize) -> Self {
        Self {
            index,
            total,
            path: path.to_path_buf(),
            extension: path.extension().map_or_else(
                || "bin".to_string(),
                |s| s.to_string_lossy().to_string().to_lowercase(),
            ),
            suffix: None,
            datetime: FileDateTime::new(),
            hash: get_file_md5(path).unwrap(),
        }
    }

    pub fn set_suffix(&mut self, suffix: Option<&str>) {
        // 保留原始后缀
        if CONFIG.retain_suffix.contains(&self.extension) {
            self.suffix = Some(self.extension.clone());
            return;
        }

        if let Some(s) = suffix {
            self.suffix = Some(s.to_string());
        } else {
            self.suffix = Some(self.extension.clone());
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
            self.suffix.as_ref().map_or(&self.extension, |s| &s)
        )
    }

    fn get_copy_path(&self, output: impl AsRef<Path>) -> PathBuf {
        // new dest path like `output/2024/12`
        let dst = PathBuf::from_iter([
            output.as_ref(),
            self.datetime.year.to_string().as_ref(),
            format!("{:02}", self.datetime.month).as_ref(),
        ]);

        // create dest path dirtory
        if !dst.is_dir() {
            fs::create_dir_all(&dst).unwrap();
        }

        dst.join(&self.get_name())
    }

    async fn datetime_from_metedata(&mut self) -> Option<Vec<FileDateTime>> {
        let mut dts: Vec<FileDateTime> = Vec::new();

        let texts = match METADATA.read(&self.path).await {
            Ok(texts) => {
                if texts.len() == 0 {
                    error!("no metadata found for {:?}", self.path);
                    return None;
                }
                texts
            }
            Err(e) => {
                error!("read metadata {:?} failed with error: {}", self.path, e);
                return None;
            }
        };

        'outer: for value in texts {
            debug!("{}", value);
            // println!("> {}", value);

            for black_str in &CONFIG.blacklist {
                if value.contains(black_str) {
                    debug!("💡 {} contains black string {}, skip...", value, black_str);
                    continue 'outer;
                }
            }
            // capture file extension from metadata
            if self.suffix.is_none() {
                if let Some(t) = capture_type(&value) {
                    debug!(
                        "🏷️ capture file extension from metadata: {}, {:?}",
                        t, self.path
                    );
                    // println!("capture file extension from metadata: {}", t);
                    self.set_suffix(Some(&t));
                }
            }

            // get date from metadata
            if let Some(dt) = get_datetime_from_string(&value) {
                debug!("{} -> {}", value, dt);
                if dt.year < 1975 {
                    warn!("💡 {} < 1975, {:?} skip...", dt.year, self.path);
                } else {
                    dts.push(dt);
                }
            }
        }

        if dts.len() == 0 {
            return None;
        }
        Some(dts)
    }

    async fn get_all_datetime(&mut self, dup_sort: bool) -> Vec<FileDateTime> {
        let mut dts = if let Some(dts) = self.datetime_from_metedata().await {
            debug!("✨ success get date from metadata: {:?}", dts);
            dts
        } else {
            panic_with_test();
            vec![]
        };

        if let Some(dt) = get_earliest_datetime_from_attributes(&self.path) {
            debug!("✨ success get date(earliest) from attributes: {}", dt);
            dts.push(dt);
        } else {
            panic_with_test();
        }

        if let Some(dt) = get_datetime_from_additional(&self.path) {
            debug!("✨ success get date from additional: {}", dt);
            dts.push(dt);
        }

        // println!("dts: {:?}", dts);
        assert!(dts.len() > 0, "💥 no date found in {:?}", self.path);

        if dup_sort {
            // sort by timestamp
            dts.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
            // remove duplicate by timestamp
            dts.dedup_by_key(|k| k.timestamp);
            // println!("sort and dedup dts: {:?}", dts);
        }

        dts
    }

    fn copy_time(&self, from: &PathBuf, to: &PathBuf) -> Result<()> {
        let times = match get_datatime_from_metadata(from) {
            Some(t) => t,
            None => {
                return Ok(());
            }
        };
        assert!(times.len() >= 2);
        let atime = times[0];
        // TODO: 修改创建时间为最小暂时未实现，当前将修改时间设置为最小的时间
        let mtime = if let Some(t) = times.iter().min() {
            *t
        } else {
            times[1]
        };
        let dest = fs::File::options().write(true).open(&to)?;
        dest.set_times(fs::FileTimes::new().set_accessed(atime).set_modified(mtime))?;
        Ok(())
    }

    pub fn copy(&self, output: impl AsRef<Path>) -> PathBuf {
        let dst = self.get_copy_path(output);
        if dst.is_file() {
            warn!(
                "💡[{}/{}] skip already exists {} -> {}",
                self.index,
                self.total,
                self.path.display(),
                dst.display()
            );
            Checker::new(&self.path).set_placed().unwrap();
            return dst;
        }

        if unsafe { ISTEST } {
            info!(
                "✅ [{}/{}] [TEST] skip copy {:?} -> {:?}",
                self.index, self.total, self.path, dst
            );
            return dst;
        }

        // copy file
        fs::copy(&self.path, &dst).unwrap();
        // copy datetime
        self.copy_time(&self.path, &dst).unwrap();

        // set placed file
        Checker::new(&self.path).set_placed().unwrap();
        info!(
            "🚚 [{}/{}] success copy {:?} -> {:?}",
            self.index, self.total, self.path, dst
        );

        dst
    }

    // #[tracing::instrument]
    pub async fn process(mut self, output: Option<&PathBuf>) -> Result<Self> {
        debug!("[{}/{}] process {:?}", self.index, self.total, self.path);

        let dts = self.get_all_datetime(true).await;
        if dts.len() == 1 {
            self.datetime = dts[0].clone();
        }

        // 处理相同日期，但时间是 00:00:00 的情况
        for index in 0..dts.len() {
            // hour, minute, second not all zero, used it
            if dts[index].hour != 0 || dts[index].minute != 0 || dts[index].second != 0 {
                self.datetime = dts[index].clone();
                break;
            }
            // if next date is not same day, used it
            if dts[index + 1].year != dts[index].year
                || dts[index + 1].month != dts[index].month
                || dts[index + 1].day != dts[index].day
            {
                self.datetime = dts[index].clone();
                break;
            }
            // if next date is same day but hour not all zero, used next date
            if dts[index + 1].hour != 0 || dts[index + 1].minute != 0 || dts[index + 1].second != 0
            {
                self.datetime = dts[index + 1].clone();
                break;
            }
        }

        if let Some(output) = output {
            self.copy(output);
        }

        Ok(self)
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
    fn test_target_new() {
        let path = get_root().join("tests/simple.jpg");
        println!("path: {:?}", path);
        let target = Target::new(&path, 1, 1);
        println!("{}", target);
        assert_eq!(target.path, path);
        assert_eq!(target.extension, "jpg");
    }

    #[test]
    fn test_target_get_name() {
        let path = get_root().join("tests/simple.jpg");
        let target = Target::new(&path, 1, 1);
        let name = target.get_name();
        println!("target: {}, name: {}", target, name);
        let check = format!("{}.{}.jpg", target.datetime.second, target.hash);
        println!("check: {}", check);
        assert!(name.contains(&check));
        let name = target.get_name();
        println!("target: {}, name: {}", target, name);
        assert!(name.contains("a18932e314dbb4c81c6fd0e282d81d16.jpg"));
    }

    #[tokio::test]
    async fn test_date_from_metedata() {
        let path = get_root().join("tests/simple.jpg.png");
        let mut target = Target::new(&path, 1, 1);
        let dts = target.datetime_from_metedata().await.unwrap();
        println!("dts: {:?}", dts);
        assert!(dts.len() == 4);
        println!("target: {:#?}", target);
        assert!(target.extension == "png");
        assert!(target.suffix == Some("jpg".to_string()));
    }

    #[tokio::test]
    async fn test_get_all_datetime() {
        let path = get_root().join("tests/simple.jpg.png");
        let mut target = Target::new(&path, 1, 1);
        let dts = target.get_all_datetime(false).await;
        println!("dts: {:#?}", dts);
        assert!(dts.len() == 5);
        let mut sorts = vec![];
        for index in 0..dts.len() - 1 {
            if dts[index].timestamp < dts[index + 1].timestamp {
                sorts.push(true);
            } else {
                sorts.push(false);
            }
        }
        println!("sorts: {:?}", sorts);
        assert!(!sorts.iter().all(|x| *x));

        let dts = target.get_all_datetime(true).await;
        println!("dts: {:#?}", dts);
        assert!(dts.len() == 3);
        assert!(dts[0].timestamp < dts[1].timestamp);
        assert!(dts[1].timestamp < dts[2].timestamp);
    }

    #[tokio::test]
    async fn test_process() {
        let path = get_root().join("tests/simple.jpg");
        let output = get_root().join("tests/output");
        let target = Target::new(&path, 1, 1).process(None).await.unwrap();
        println!("target: {:#?}", target);
        assert!(target.datetime.timestamp == 1037460421);

        let dst = target.copy(&output);
        println!("copy from {:?} to {:?}", &path, &dst);
        assert!(dst.is_file());

        let src_meta = std::fs::metadata(&path).unwrap();
        let dst_meta = std::fs::metadata(&dst).unwrap();
        println!("src_meta: {:#?}", src_meta);
        println!("dst_meta: {:#?}", dst_meta);

        // std::fs::remove_dir_all(&output).unwrap();
    }
}
