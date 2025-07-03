use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use tracing::{debug, debug_span, info};
use tracing_futures::Instrument;
use walkdir::WalkDir;

use config::CONFIG;

static mut IS_TEST: bool = false;

struct FileDateTime {
    path: PathBuf,
    datetimes: Vec<DateTime<Utc>>,
}

pub async fn do_process(input: &Path, output: Option<&Path>, test: bool) -> Result<()> {
    unsafe {
        IS_TEST = test;
    }
    let output = if let Some(o) = output {
        &o.canonicalize()?
    } else {
        &input.with_extension("mmfplace").canonicalize()?
    };
    if !output.is_dir() {
        std::fs::create_dir_all(output)?;
    }

    let total = WalkDir::new(input)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .count();
    info!(input=?input, total=total, output=?output, test=test, "start process");

    let concurrency: usize = CONFIG.batch_size;
    let channel_size: usize = 100;
    let (tx, mut rx) = mpsc::channel::<FileDateTime>(channel_size);
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
                        if let Err(e) = process_file(fdt, &processed_count).await {
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
                            let datetimes = get_datetimes(&path).await.unwrap_or(vec![]);
                            if tx.send(FileDateTime { path, datetimes }).await.is_err() {
                                debug!("通道已关闭，无法发送文件");
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

async fn get_datetimes(path: &PathBuf) -> Result<Vec<DateTime<Utc>>> {
    // todo: process file
    debug!("begin get_datetimes: {:?}", path);
    sleep(Duration::from_millis(2)).await;
    debug!("end get_datetimes: {:?}", path);
    Ok(vec![])
}

async fn process_file(fdt: FileDateTime, processed_count: &Arc<AtomicUsize>) -> Result<()> {
    debug!(
        "begin processing file: {:?}, count: {:?}",
        fdt.path, processed_count
    );
    sleep(Duration::from_millis(3)).await;
    let count = processed_count.fetch_add(1, Ordering::SeqCst) + 1;
    debug!("finish processing file: {:?}, count: {}", fdt.path, count);
    Ok(())
}
