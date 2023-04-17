use crate::config::Config;
use crate::place::parse::FileParser;
use anyhow::{anyhow, Result};
use base16ct;
use filetime::{set_file_times, FileTime};
use meta::MetadataReader;
use sha2::{Digest, Sha256};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;

pub mod meta;
pub mod parse;

/// calculates sha256 digest as lowercase hex string
fn sha256_digest(path: impl AsRef<Path>) -> Result<String> {
    let input = std::fs::File::open(path)?;
    let mut reader = BufReader::new(input);

    let digest = {
        let mut hasher = Sha256::new();
        let mut buffer = [0; 1024];
        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        hasher.finalize()
    };
    let hex_hash = base16ct::lower::encode_string(&digest);
    Ok(hex_hash)
}

fn get_copy_path(
    output: impl AsRef<Path>,
    file_parser: &FileParser,
    dup_max: u32,
) -> Result<PathBuf> {
    let file_dir = output
        .as_ref()
        .clone()
        .join(file_parser.datetime.get_year().to_string());

    for index in 0..dup_max {
        let file_name = file_parser.get_name(index);
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
        let src_hash = sha256_digest(&file_parser.file_path)?;
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

async fn copy_to<T>(src: T, dst: T) -> Result<bool>
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
                tokio::fs::create_dir_all(parent).await?;
            }
        }
        None => {
            return Err(anyhow!("get parent directory failed {:?}", copy_to));
        }
    }

    tokio::fs::copy(&copy_from, &copy_to).await?;

    let metadata = std::fs::metadata(&copy_from)?;
    // log::info!("src file time: {:#?}", metadata);
    let atime = FileTime::from_last_access_time(&metadata);
    let mtime = FileTime::from_last_modification_time(&metadata);
    set_file_times(&copy_to, atime, mtime)?;
    // let metadata = std::fs::metadata(&copy_to)?;
    // log::info!("dst file time: {:#?}", metadata);
    Ok(true)
}

pub async fn process<T>(
    work_dir: T,
    input: T,
    output: T,
    config: Config,
    test: bool,
) -> Result<bool>
where
    T: AsRef<Path>,
{
    let total: u64 = match input.as_ref().is_dir() {
        true => WalkDir::new(input.as_ref())
            .into_iter()
            .filter(|e| e.as_ref().unwrap().file_type().is_file())
            .count() as u64,
        false => 1,
    };

    log::info!(
        "start splits process with:\n  work_dir: {:?}\n  input: {:?}\n  output: {:?}\n  test: {}\n  total: {}",
        work_dir.as_ref(),
        input.as_ref(),
        output.as_ref(),
        test, total
    );
    let dup_max = config.dup_max;
    let arc_config = Arc::new(config);
    let arc_mreader = Arc::new(MetadataReader::new(&work_dir)?);
    let mut index: u64 = 0;

    let mut handles = Vec::new();
    for entry in WalkDir::new(input) {
        let entry = entry?;
        if entry.file_type().is_file() {
            index += 1;
            log::info!(
                "[{}/{}] begin processing file: {:?}",
                index,
                total,
                &entry.path()
            );
            let _config = Arc::clone(&arc_config);
            let _reader = Arc::clone(&arc_mreader);
            handles.push(tokio::spawn(async move {
                let file_parser = parse::FileParser::new(&entry.path())
                    .create(&_config, &_reader)
                    .await
                    .unwrap();
                file_parser
            }));
        }
    }

    for handle in handles {
        let file_parser = handle.await?;
        log::info!("parser as: {:#?}", file_parser);
        let copy_path = get_copy_path(&output, &file_parser, dup_max)?;
        if test {
            log::info!(
                "[{}/{}] [Success] test without copy {:?} to {:?}",
                index,
                total,
                file_parser.file_path,
                copy_path
            );
        } else {
            copy_to(&file_parser.file_path, &copy_path).await?;
            log::info!(
                "[{}/{}] [Success] copy {:?} to {:?}",
                index,
                total,
                file_parser.file_path,
                copy_path
            );
        }
    }

    Ok(true)
}
