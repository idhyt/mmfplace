use anyhow::{anyhow, Result};
use filetime::{set_file_times, FileTime};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;

use super::meta::MetadataReader;
use super::pick::PickFile;
use config::CONFIG;
use utils::crypto::sha256_digest;

pub async fn process(input: &PathBuf, test: bool) -> Result<()> {
    let config = CONFIG.lock().await;

    let total: u64 = match input.is_dir() {
        true => WalkDir::new(input)
            .into_iter()
            .filter(|e| e.as_ref().unwrap().file_type().is_file())
            .count() as u64,
        false => 1,
    };

    log::info!(
        "start process with:\n  work_dir: {:?}\n  input: {:?}\n  output: {:?}\n  test: {}\n  total: {}",
        config.work_dir,
        input,
        config.output,
        test,
        total
    );

    let reader = MetadataReader::new(&config.tools_dir).await?;
    log::debug!("reader: {:#?}", reader);

    let config = Arc::new(config);
    let mreader = Arc::new(reader);

    let mut index = 0;
    let mut handles = Vec::new();

    for entry in WalkDir::new(input) {
        let entry = entry?;
        if entry.file_type().is_file() {
            index += 1;
            let _config = Arc::clone(&config);
            let _reader = Arc::clone(&mreader);
            handles.push(tokio::spawn(async move {
                let pf = PickFile::new(&entry.path().to_path_buf(), index, total);
                return pf.create(&_config.parser, &_reader).await.unwrap();
                // process_one(pf, &_config, &_reader, test).await.unwrap();
            }));
        }
        // if handles len is bigger than config.parser.batch_size, wait for all handles done and clear it
        if handles.len() as u32 >= config.parser.batch_size {
            for handle in handles.iter_mut() {
                let pf = handle.await?;
                do_copy(pf, &config.output, config.parser.dup_max, test)?;
            }
            handles.clear();
        }
    }

    // wait for all handles done
    if handles.len() > 0 {
        for handle in handles {
            let pf = handle.await?;
            do_copy(pf, &config.output, config.parser.dup_max, test)?;
        }
    }

    Ok(())
}

// async fn process_one(
//     pf: PickFile,
//     config: &config::Config,
//     reader: &MetadataReader,
//     test: bool,
// ) -> Result<()> {
//     let pf = pf.create(&config.parser, reader).await?;
//     log::info!("pickup file: {:#?}", pf);

//     let date_path = get_date_path(&config.output, &pf, config.parser.dup_max)?;
//     if test {
//         log::info!(
//             "[{}/{}] [Success] test without copy {:?} to {:?}",
//             pf.index,
//             pf.total,
//             pf.fi.file_path,
//             date_path
//         );
//     } else {
//         // copy_to(&pf.fi.file_path, &date_path).await?;
//         copy_to(&pf.fi.file_path, &date_path)?;
//         log::info!(
//             "[{}/{}] [Success] copy {:?} to {:?}",
//             pf.index,
//             pf.total,
//             pf.fi.file_path,
//             date_path
//         );
//     }
//     Ok(())
// }

fn do_copy(pf: PickFile, output: &PathBuf, dup_max: u32, test: bool) -> Result<()> {
    let date_path = get_date_path(output, &pf, dup_max)?;
    if test {
        log::info!(
            "[{}/{}] [Success] test without copy {:?} to {:?}",
            pf.index,
            pf.total,
            pf.fi.file_path,
            date_path
        );
    } else {
        // copy_to(&pf.fi.file_path, &date_path).await?;
        copy_to(&pf.fi.file_path, &date_path)?;
        log::info!(
            "[{}/{}] [Success] copy {:?} to {:?}",
            pf.index,
            pf.total,
            pf.fi.file_path,
            date_path
        );
    }
    Ok(())
}

fn get_date_path(output: impl AsRef<Path>, pickf: &PickFile, dup_max: u32) -> Result<PathBuf> {
    let file_dir = output.as_ref().join(pickf.fi.datetime.year.to_string());

    for index in 0..dup_max {
        let file_name = pickf.fi.get_name(index);
        // directory not exist.
        if !file_dir.is_dir() {
            return Ok(file_dir.join(file_name));
        }
        // file not exist
        let file_path = file_dir.join(file_name);
        if !file_path.is_file() {
            return Ok(file_path);
        }
        // file exist, check hash
        let src_hash = sha256_digest(&pickf.fi.file_path)?;
        let dst_hash = sha256_digest(&file_path)?;
        // hash match, skip
        if src_hash == dst_hash {
            log::debug!(
                "same hash file {:?} exists, skip, src: {}, dst: {}",
                file_path,
                src_hash,
                dst_hash
            );
            return Ok(file_path);
        }
        // hash not match, try next
        log::debug!(
            "file {:?} exists, but hash not match, try next, src: {}, dst: {}",
            file_path,
            src_hash,
            dst_hash
        );
    }
    Err(anyhow!("try {} time but file exists", dup_max))
}

// async fn copy_to<T>(src: T, dst: T) -> Result<bool>
fn copy_to<T>(src: T, dst: T) -> Result<bool>
where
    T: AsRef<Path>,
{
    let copy_from = src.as_ref();
    let copy_to = dst.as_ref();

    if copy_to.is_file() {
        log::info!(
            "[Duplicate] copy {:?} to {:?} exists, skip",
            copy_from,
            copy_to
        );
        return Ok(true);
    }

    match copy_to.parent() {
        Some(parent) => {
            if !parent.is_dir() {
                // tokio::fs::create_dir_all(parent).await?;
                std::fs::create_dir_all(parent)?;
            }
        }
        None => {
            return Err(anyhow!("get parent directory failed {:?}", copy_to));
        }
    }

    // bug will cause panic at `one file used by other` in async process...
    // tokio::fs::copy(&copy_from, &copy_to).await?;
    std::fs::copy(&copy_from, &copy_to)?;

    let metadata = std::fs::metadata(&copy_from)?;
    // log::info!("src file time: {:#?}", metadata);
    let atime = FileTime::from_last_access_time(&metadata);
    let mtime = FileTime::from_last_modification_time(&metadata);
    set_file_times(&copy_to, atime, mtime)?;
    // let metadata = std::fs::metadata(&copy_to)?;
    // log::info!("dst file time: {:#?}", metadata);
    Ok(true)
}

