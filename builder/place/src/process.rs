use anyhow::Result;
use chrono::Datelike;
use once_cell::sync::OnceCell;
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Semaphore;
use tracing::{debug, debug_span, error, info, warn};
use tracing_futures::Instrument;
use walkdir::WalkDir;

use super::db::{get_connection, insert_finfo, query_finfo, update_finfo, FileInfo};
use super::target::{Target, OUTPUT_GEN};

use config::CONFIG;
use tools::metadata_extractor;

fn error_with_exit() -> ! {
    std::process::exit(1);
}

// 临时共享数据，我不知道该取什么名字hhh...
#[derive(Debug, Clone, Default)]
struct TempData {
    input: PathBuf,
    output: PathBuf,
    test: bool,
    rename: bool,
    total: usize,
}

static TEMPDATA: OnceCell<TempData> = OnceCell::new();

fn temp_init(input: PathBuf, output: PathBuf, test: bool, rename: bool, total: usize) {
    TEMPDATA
        .set(TempData {
            input,
            output,
            test,
            rename,
            total,
        })
        .expect("TempData is already initialized")
}
fn temp_get() -> &'static TempData {
    TEMPDATA.get().expect("TempData is not initialized")
}

pub async fn do_process(
    input: PathBuf,
    output: PathBuf,
    test: bool,
    rename_with_ymd: bool,
) -> Result<()> {
    // let (input, output, test) = (&temp_get().input, &temp_get().output, temp_get().test);
    let total = walkdir::WalkDir::new(&input)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .count();
    temp_init(input, output, test, rename_with_ymd, total);
    let (input, output, test) = (&temp_get().input, &temp_get().output, temp_get().test);
    info!(input=?input, total=total, output=?output, test=test, "start process");

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
                let span = debug_span!("task_place", file = ?fdt.path);
                async {
                    if let Err(e) = do_place(fdt, &processed_count).await {
                        error!(error=%e, "place error");
                        error_with_exit();
                    }
                }
                .instrument(span)
                .await;
            }
            info!("finished consumer");
        }
        .instrument(root_span)
    });

    let producer = tokio::spawn({
        // let input = temp_get().input.clone();
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
                        let span = debug_span!("task_parse", file = ?path);
                        async {
                            let _permit = semaphore.acquire().await.unwrap();
                            match do_parse(path).await {
                                Ok(t) => {
                                    if let Err(e) = tx.send(t).await {
                                        error!("send task error: {:#?}", e);
                                        error_with_exit();
                                    }
                                }
                                Err(e) => {
                                    error!(error=%e, "parse error");
                                    error_with_exit();
                                }
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
                    let _ = futures::future::join_all(tasks).await;
                    tasks = Vec::new();
                }
            }
            futures::future::join_all(tasks).await;
            // drop(tx);
            info!("finished producer");
        }
    });

    // producer.await?;
    // consumer.await?;
    let _ = tokio::join!(producer, consumer);

    info!("all done");
    Ok(())
}

// 计算文件hash -> 判断hash是否在数据库中 -> 存在 -> 获取parts部分拼接路径是否存在 -> 存在跳过/不存在拷贝
//                                      -> 不存在 -> 解析所有时间(元数据+文件属性) -> 取最早 -> 插入数据库 -> 拷贝文件
async fn do_parse(path: PathBuf) -> Result<Target> {
    debug!(file=?path, "🚀 begin parse file");
    let mut target = Target::new(path)?;

    // if test mode, don't check exists
    if temp_get().test {
        debug!(file=?target.path, "💡 test mode, skip exists check");
    } else {
        let conn = get_connection().lock().unwrap();
        if let Some(history) = query_finfo(&conn, &target.hash)? {
            target.dealt = true;
            target.parts = Some(history.parts.into());
            // 更新 earliest，后边需要设置文件属性时间
            target.set_earliest(Some(history.earliest as u64))?;
        }
    }

    // 如果查到，说明之前已处理过了，则不再进行元数据解析
    if target.dealt {
        debug!(file = ?target.path, "file is already dealt before");
        return Ok(target);
    }

    // 是否需要获取文件类型
    let captype = CONFIG
        .typeregex
        .ignore
        .as_ref()
        .map_or(true, |ignore| !ignore.contains(&target.extension));
    // 如果需要忽略，则设置type字段，后边逻辑将跳过获取文件类型
    if !captype {
        debug!(file = ?target.path, "💡 the file type is ignored");
        target.ftype = Some(target.extension.clone());
    }

    // 获取文件元数据并解析出所有时间格式
    let texts = metadata_extractor(&target.path).await?;
    'outer: for text in texts.iter() {
        // 过滤字符串
        if let Some(ignore) = &CONFIG.dateregex.ignore {
            for black in ignore {
                if text.contains(black) {
                    debug!(black = black, text = text, "skip black string");
                    continue 'outer;
                }
            }
        }

        // 获取文件type
        if target.ftype.is_none() {
            for capture in &CONFIG.typeregex.list {
                if let Ok(t) = capture.capture(text) {
                    info!(
                        text = text,
                        ftype = t,
                        "🎉 success parse filetype from text"
                    );
                    target.ftype = Some(t);
                    break;
                }
            }
        }

        // 获取文件时间
        if let Ok(dt) = dateparser::parse(text) {
            if dt.year() < 1975 {
                warn!(file=?target.path, datetime=%dt, "💡 skip the datetime < 1975");
            } else {
                info!(text = text, datetime = %dt, "🎉 success parse datetime from text");
                target.tinfo.parsedtimes.push(dt);
            }
        }
    }
    target.set_earliest(None)?;

    Ok(target)
}

async fn do_place(mut target: Target, processed_count: &Arc<AtomicUsize>) -> Result<()> {
    let count = processed_count.fetch_add(1, Ordering::SeqCst) + 1;
    let total = temp_get().total;
    debug!(file=?target.path, "🚀 begin place {} file", count);
    target.set_output(&temp_get().output, temp_get().rename)?;

    if temp_get().test {
        info!(from=?target.path, to=?target.output, "✅ [{count}/{total}] success test finish");
        return Ok(());
    }

    // 在解析阶段，如果在数据库中找打同 hash，说明之前处理过了，会标记字段 dealt=true，并使用处理过的 parts 作为路径
    // 当字段 dealt=false 时，说明当前走了解析流程
    // 但是在并发过程中，会存在相同 hash 的 /path/to/A 和 /path/to/B 同时被处理

    // 没有走 parse 流程，使用的历史 parts, 数据库不需要处理，直接拷贝即可
    if target.dealt {
        target.copy_with_times()?;
        info!(from=?target.path, to=?target.output, "✅ [{count}/{total}] success place with history parsed finish");
        return Ok(());
    }

    // 处理并发中可能存在同 hash
    {
        let parts = target.parts.as_ref().unwrap();
        let finfo = FileInfo {
            parts: Cow::Borrowed(parts),
            hash: Cow::Borrowed(&target.hash),
            earliest: target.get_earliest()?.timestamp(),
        };
        let conn = get_connection().lock().unwrap();
        // 先查是否存在
        let find = query_finfo(&conn, &target.hash)?;
        if find.is_none() {
            // 不存在直接插入数据库即可
            insert_finfo(&conn, &finfo).map_err(|e| {
                anyhow::anyhow!(
                    "insert hash error file={:?}, hash={} error={:?}",
                    target.path,
                    target.hash,
                    e
                )
            })?;
            target.copy_with_times()?;
            info!(from=?target.path, to=?target.output, "✅ [{count}/{total}] success place with new parsed finish");
            return Ok(());
        }

        let history = find.unwrap();
        info!(current=?parts, history=?history.parts, "same hash file found, compare the time and overwrite it");
        let history_file = OUTPUT_GEN(&temp_get().output, &history.parts.to_vec());
        // 如果已经存在了，比对 eraiest time，如果当前的更早，则更新，否则直接丢弃
        if finfo.earliest < history.earliest {
            // 删除原来的文件
            if history_file.is_file() {
                std::fs::remove_file(&history_file)?;
            }
            // 更新数据库
            update_finfo(&conn, &finfo)?;
            target.copy_with_times()?;
            info!(from=?target.path, to=?target.output, "✅ [{count}/{total}] success place (<history) update finish");
        }
        // 时间晚，则丢弃
        else {
            // 检查下原始文件是否存在，如果不存在，则需要复制过去
            // 更新 output
            target.output = history_file;
            if !target.output.is_file() {
                warn!(file=?target.output, "⚠️ history file not exists, restore it");
                // 设置 earliest
                target.set_earliest(Some(history.earliest as u64))?;
                // 复制
                target.copy_with_times()?;
            }
            info!(from=?target.path, to=?target.output, "✅ [{count}/{total}] success place (>=history) finish");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::prelude::*;
    use chrono::Utc;

    fn get_root() -> PathBuf {
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    #[tokio::test]
    async fn test_do_parse() {
        let tests = get_root().join("tests");
        let input = tests.join("2002/11/simple.png");
        let output = get_root().join("tests");
        temp_init(input.clone(), output.clone(), true, false, 1);
        let mut target = do_parse(input.clone()).await.unwrap();
        println!("target: {:#?}", target);
        assert_eq!("simple", target.name);
        assert_eq!("png", target.extension);
        assert_eq!(Some("jpg".to_string()), target.ftype);
        assert_eq!(target.tinfo.parsedtimes.len(), 3);
        assert_eq!(target.hash, "a18932e314dbb4c81c6fd0e282d81d16");
        assert_eq!(
            target.get_earliest().unwrap(),
            Utc.with_ymd_and_hms(2002, 11, 16, 0, 0, 0).unwrap()
        );
        assert!(target.tinfo.attrtimes.len() >= 2);

        target.set_output(&output, false).unwrap();
        let copy_path = target.output.clone();
        println!("copy_path: {:?}", copy_path);
        assert_eq!(copy_path, output.join("2002/11/simple_01.jpg"));
        assert_eq!(target.parts.unwrap(), vec!["2002", "11", "simple_01.jpg"]);

        // now we copy to a new file
        let dup_file = input.with_file_name("simple_01.jpg");
        std::fs::copy(&input, &dup_file).unwrap();
        let input = tests.join("2002/11/simple.jpg");
        let mut target = do_parse(input.clone()).await.unwrap();
        println!("new target: {:#?}", target);
        assert_eq!(target.hash, "a18932e314dbb4c81c6fd0e282d81d16");
        assert_eq!("simple", target.name);
        assert_eq!("jpg", target.extension);
        assert_eq!(Some("jpg".to_string()), target.ftype);
        assert_eq!(
            target.get_earliest().unwrap(),
            Utc.with_ymd_and_hms(2002, 11, 16, 0, 0, 0).unwrap()
        );

        target.set_output(&output, false).unwrap();
        let copy_path = target.output.clone();
        println!("copy_path: {:?}", copy_path);
        assert_eq!(copy_path, output.join("2002/11/simple_02.jpg"));
        assert_eq!(
            *target.parts.as_ref().unwrap(),
            vec!["2002", "11", "simple_02.jpg"]
        );

        target.set_output(&output, true).unwrap();
        let copy_path = target.output.clone();
        assert_eq!(copy_path, output.join("2002/11/2002-11-16.jpg"));

        std::fs::remove_file(&dup_file).unwrap();
    }
}
