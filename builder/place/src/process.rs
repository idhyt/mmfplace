use anyhow::Result;
use chrono::Datelike;
use chrono::{DateTime, Utc};
use once_cell::sync::OnceCell;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::mpsc;
use tokio::sync::Semaphore;
use tracing::{debug, debug_span, error, info, warn};
use tracing_futures::Instrument;
use walkdir::WalkDir;

use crate::db::{get_connection, insert_hash, query_parts, FileHash};

use config::CONFIG;
use tools::metadata_extractor;
use utils::crypto::get_file_md5;

#[derive(Debug, Clone, Default)]
struct Target {
    // target file path
    path: PathBuf,
    // parsed datetime from metadata
    datetimes: Vec<DateTime<Utc>>,
    // hash with md5
    hash: String,
    // the original file
    extension: String,
    // the file name without extension
    name: String,
    // the file parsed type
    pub type_: Option<String>,
    // the earliest datetime
    earliest: DateTime<Utc>,
    // datetime from file attributes
    // [accessed, modified, created]
    attrtimes: Vec<Option<SystemTime>>,
}

// 临时共享数据，我不知道该取什么名字hhh...
#[derive(Debug, Clone, Default)]
struct TempData {
    test: bool,
    output: PathBuf,
}

static TEMPDATA: OnceCell<TempData> = OnceCell::new();

fn temp_init(test: bool, output: PathBuf) {
    TEMPDATA
        .set(TempData { test, output })
        .expect("TempData is already initialized")
}
fn temp_get() -> &'static TempData {
    TEMPDATA.get().expect("TempData is not initialized")
}

pub async fn do_process(input: &Path, output: Option<&Path>, test: bool) -> Result<()> {
    let output = if let Some(o) = output {
        o.canonicalize()?
    } else {
        input.with_extension("mmfplace").canonicalize()?
    };
    if !output.is_dir() {
        std::fs::create_dir_all(&output)?;
    }
    let total = WalkDir::new(input)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .count();
    info!(input=?input, total=total, output=?output, test=test, "start process");
    // init temp data
    temp_init(test, output);

    // MPSC mode
    let concurrency: usize = CONFIG.batch_size;
    let channel_size: usize = 100;
    let (tx, mut rx) = mpsc::channel::<Target>(channel_size);
    let processed_count = Arc::new(AtomicUsize::new(0));
    let semaphore = Arc::new(Semaphore::new(concurrency));

    let root_span = debug_span!("process");
    let _enter = root_span.enter();

    let consumer = tokio::spawn({
        let processed_count = Arc::clone(&processed_count);
        let root_span = root_span.clone();
        async move {
            while let Some(fdt) = rx.recv().await {
                let span = debug_span!("task_place", path = ?fdt.path);
                async {
                    while let Some(fdt) = rx.recv().await {
                        if let Err(e) = do_place(fdt, &processed_count).await {
                            eprintln!("处理文件失败: {}", e);
                        }
                    }
                }
                .instrument(span)
                .await;
            }
            info!("finished");
        }
        .instrument(root_span)
    });

    let producer = tokio::spawn({
        let input = input.to_path_buf();
        let tx = tx; // tx.clone();
        let semaphore = Arc::clone(&semaphore);
        let root_span = root_span.clone();

        async move {
            let mut tasks = Vec::new();
            for entry in WalkDir::new(input)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path().to_path_buf();
                let tx = tx.clone();
                let semaphore = Arc::clone(&semaphore);
                let root_span = root_span.clone();

                let task = tokio::spawn(
                    async move {
                        let span = debug_span!("task_parse", path = ?path);
                        async {
                            let _permit = semaphore.acquire().await.unwrap();
                            if let Some(t) = do_parse(path).await.unwrap() {
                                if tx.send(t).await.is_err() {
                                    debug!("通道已关闭，无法发送文件");
                                }
                            } else {
                                // 已经处理过的文件，将忽略
                            }
                            // drop(_permit);
                        }
                        .instrument(span)
                        .await
                    }
                    .instrument(root_span),
                );

                tasks.push(task);

                if tasks.len() >= channel_size {
                    futures::future::join_all(tasks).await;
                    tasks = Vec::new();
                }
            }

            futures::future::join_all(tasks).await;

            drop(tx);
        }
    });

    // producer.await?;
    // consumer.await?;
    let _ = tokio::join!(producer, consumer);

    info!("所有文件处理完成");

    Ok(())
}

// 计算文件hash -> 判断hash是否在数据库中 -> 存在 -> 获取parts部分拼接路径是否存在 -> 存在跳过/不存在拷贝
//                                      -> 不存在 -> 解析所有时间(元数据+文件属性) -> 取最早 -> 插入数据库 -> 拷贝文件
async fn do_parse(path: PathBuf) -> Result<Option<Target>> {
    info!("🚀 begin parse file: {:?}", path);
    let target = Target::new(path);
    let parts = {
        let conn = get_connection().lock().unwrap();
        query_parts(&conn, &target.hash)?
    };
    // 如果查到，说明之前已处理过了，则不再进行元数据解析
    if parts.is_some() {
        debug!(hash = target.hash, "file is already parsed");
        return Ok(Some(target));
    }

    let mut target = target;
    // 获取文件元数据并解析出所有时间格式
    let texts = metadata_extractor(&target.path).await?;
    'outer: for text in texts.iter() {
        // 过滤字符串
        for black in &CONFIG.blacklist {
            if text.contains(black) {
                debug!(black = black, text = text, "skip black string");
                continue 'outer;
            }
        }
        // // TODO: 获取文件type
        // if target.type_.is_none() {
        //     if let Some(t) = capture_type(&value) {
        //         info!(
        //             path=?target.path,
        //             type_ = t,
        //             "🏷️ success capture file type from metadata",
        //         );
        //         // println!("capture file extension from metadata: {}", t);
        //         target.type_ = t;
        //     }
        // }
        if let Ok(dt) = dateparser::parse(text) {
            if dt.year() < 1975 {
                warn!(path=?target.path, datetime=%dt, "💡 skip the datetime < 1975");
            } else {
                info!(text = text, datetime = %dt, "success date parse");
                target.add_datetime(dt);
            }
        }
    }
    target.set_earliest();

    Ok(Some(target))
}

async fn do_place(target: Target, processed_count: &Arc<AtomicUsize>) -> Result<()> {
    let count = processed_count.fetch_add(1, Ordering::SeqCst) + 1;
    debug!(
        "🚚 begin place file: {:?}, count: {:?}",
        target.path, processed_count
    );
    let generation = |o: &Path, p: &Vec<String>| {
        p.iter().fold(o.to_owned(), |mut path, p| {
            path.push(p);
            path
        })
    };
    let output = temp_get().output.to_owned();

    let (parts, exist) = {
        let find = {
            let conn = get_connection().lock().unwrap();
            query_parts(&conn, &target.hash)?
        };
        if let Some(parts) = find {
            // 找到，则说明已经处理过了
            (Some(parts), true)
        } else {
            // 没找到，生成
            let mut parts = None;
            // 有可能文件重名，循环生成
            for i in 0..1000 {
                let got = target.get_parts(i);
                let check = generation(&output, &got);
                if !check.is_file() {
                    parts = Some(got);
                    break;
                }
                debug!(
                    exist = ?check,
                    count = i+1,
                    "already exist"
                )
            }
            (parts, false)
        }
    };

    if parts.is_none() {
        return Err(anyhow::anyhow!("parts is none"));
    }
    let parts = parts.unwrap();

    let copy_path = generation(&output, &parts);
    let need_copy = {
        if copy_path.is_file() {
            if target.hash == get_file_md5(&copy_path).unwrap() {
                info!(path=?copy_path, "skip with same hash");
                false
            } else {
                warn!(path=?copy_path, "overwrite with different hash");
                true
            }
        } else {
            info!(path=?copy_path, "copy with not exist");
            true
        }
    };

    if temp_get().test {
        info!(from=?target.path, to=?copy_path, count=count, "test success finish");
        return Ok(());
    }

    if need_copy {
        copy_file_with_times(&target.path, &copy_path, &target.attrtimes)?;
    }
    if !exist {
        // 插入数据库
        let conn = get_connection().lock().unwrap();
        insert_hash(
            &conn,
            &FileHash {
                parts: &parts,
                hash: &target.hash,
            },
        )?;
        debug!(path=?copy_path, "success insert hash");
    }

    info!(from=?target.path, to=?copy_path, count=count, "success finish");
    Ok(())
}

fn copy_file_with_times(src: &Path, dst: &Path, times: &Vec<Option<SystemTime>>) -> Result<()> {
    let dir = dst.parent().unwrap();
    if !dir.exists() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::copy(src, dst)?;

    if times.len() != 3 {
        return Err(anyhow::anyhow!("the attributes time invalid! {:?}", times));
    }
    let mut new_times = std::fs::FileTimes::new();
    if let Some(atime) = times[0] {
        new_times = new_times.set_accessed(atime);
    }
    if let Some(mtime) = times[1] {
        new_times = new_times.set_accessed(mtime);
    }
    if let Some(ctime) = times[2] {
        new_times = new_times.set_accessed(ctime);
    }

    // if let Ok(atime) = src_meta.accessed() {
    //     times = times.set_accessed(atime);
    // } else {
    //     warn!(file=?src, "💡 accessed time not found");
    // }
    // if let Ok(mtime) = src_meta.modified() {
    //     times = times.set_modified(mtime);
    // } else {
    //     warn!(file=?src, "💡 modified time not found");
    // }
    // if let Ok(ctime) = src_meta.created() {
    //     times = times.set_created(ctime);
    // } else {
    //     warn!(file=?src, "💡 created time not found");
    // }
    std::fs::File::options()
        .write(true)
        .open(dst)?
        .set_times(new_times)?;
    Ok(())
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
        debug!(file=?self.path, earliest = ?self.earliest, "success set earliest datetime");
    }
}
