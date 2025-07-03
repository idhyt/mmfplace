use anyhow::Result;
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
    let (tx, mut rx) = mpsc::channel::<PathBuf>(channel_size);
    let processed_count = Arc::new(AtomicUsize::new(0));
    let semaphore = Arc::new(Semaphore::new(concurrency));

    let consumer = tokio::spawn({
        let processed_count = Arc::clone(&processed_count);
        async move {
            while let Some(path) = rx.recv().await {
                if let Err(e) = process_file(path, &processed_count).await {
                    eprintln!("处理文件失败: {}", e);
                }
            }
            info!("finished");
        }
    });

    let producer = tokio::spawn({
        let input = input.to_path_buf();
        let tx = tx; // tx.clone();
        let semaphore = Arc::clone(&semaphore);
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

                let task = tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    // todo: process file
                    sleep(Duration::from_millis(2)).await;
                    if tx.send(path).await.is_err() {
                        eprintln!("通道已关闭，无法发送文件");
                    }

                    // drop(_permit);
                });

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

async fn process_file(path: PathBuf, processed_count: &Arc<AtomicUsize>) -> Result<()> {
    info!(
        "begin processing file: {:?}, count: {:?}",
        path, processed_count
    );
    sleep(Duration::from_millis(3)).await;
    let count = processed_count.fetch_add(1, Ordering::SeqCst) + 1;
    info!("finish processing file: {:?}, count: {}", path, count);
    Ok(())
}
