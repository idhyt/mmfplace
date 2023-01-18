use anyhow::{anyhow, Result};
use base16ct;
use config::config::Config;
use extractor::metadata::MetadataReader;
use extractor::parser::{self, FileMeta};
use sha2::{Digest, Sha256};
use std::fs::{create_dir_all, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct SplitsProcess {
    // work_dir: PathBuf,
    input: PathBuf,
    output: PathBuf,
    config: Config,
    test: bool,
    extractor: MetadataReader, // only init once
}

/// calculates sha256 digest as lowercase hex string
fn sha256_digest(path: &PathBuf) -> Result<String> {
    let input = File::open(path)?;
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

impl SplitsProcess {
    pub async fn new(
        work_dir: Option<PathBuf>,
        input: impl AsRef<Path>,
        output: Option<PathBuf>,
        config: Config,
        test: bool,
    ) -> Result<Self> {
        if !input.as_ref().exists() {
            return Err(anyhow!("input path not exist!"));
        }

        let work_dir = match work_dir {
            Some(wd) => wd,
            None => {
                let mut exe_dir = std::env::current_exe()?.clone();
                exe_dir.pop();
                exe_dir
            }
        };

        let output = match output {
            Some(o) => o,
            None => work_dir.clone().join("output"),
        };

        // if !test && !output.is_dir() {
        //     create_dir_all(&output)?;
        // }

        let extractor = MetadataReader::new(None).await?;

        Ok(Self {
            // work_dir,
            input: input.as_ref().to_path_buf(),
            output: output,
            config,
            test,
            extractor,
        })
    }

    async fn get_copy_path(&self, fmeta: &parser::FileMeta) -> Result<PathBuf> {
        let file_dir = self
            .output
            .clone()
            .join(fmeta.datetime.get_year().to_string());
        for index in 0..self.config.dup_max {
            let file_name = fmeta.get_name(index as u16);
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
            let src_hash = sha256_digest(&fmeta.file_path)?;
            let dst_hash = sha256_digest(&file_path)?;
            // hash match, skip
            if src_hash == dst_hash {
                log::debug!(
                    "same hash file {} exists, skip, src: {}, dst: {}",
                    file_path.display(),
                    src_hash,
                    dst_hash
                );
                return Ok(file_path);
            }
            // hash not match, try next
            log::debug!(
                "file {} exists, but hash not match, try next, src: {}, dst: {}",
                file_path.display(),
                src_hash,
                dst_hash
            );
        }
        Err(anyhow!("try {} time but file exists", self.config.dup_max))
    }

    async fn do_split(&self, input_file: impl AsRef<Path>) -> Result<bool> {
        log::debug!("start process {}", input_file.as_ref().display());
        let fmeta = FileMeta::new(&input_file)
            .process(&self.config, &self.extractor)
            .await?;
        log::debug!("file metadata: {:#?}", fmeta);

        let copy_path = self.get_copy_path(&fmeta).await?;
        if self.test {
            log::info!(
                "[Success] test without copy {} to {}",
                fmeta.file_path.display(),
                copy_path.display()
            );
            return Ok(true);
        }

        match copy_path.is_file() {
            true => {
                log::info!(
                    "[Duplicate] copy {} to {} exists, skip",
                    fmeta.file_path.display(),
                    copy_path.display()
                );
                return Ok(true);
            }
            false => {
                match copy_path.parent() {
                    Some(parent) => {
                        if !parent.is_dir() {
                            create_dir_all(parent)?;
                        }
                    }
                    None => {
                        return Err(anyhow!(
                            "get parent directory failed {}",
                            copy_path.display()
                        ));
                    }
                }
                fmeta.copy_to(&copy_path).await?;
                log::info!(
                    "[Success] copy {} to {}",
                    fmeta.file_path.display(),
                    copy_path.display()
                );
            }
        }

        Ok(true)
    }

    pub async fn run(&self) -> Result<bool> {
        // log::info!("start splits process");
        // log::info!("input: {}", self.input.display());
        // log::info!("output: {}", self.output.display());
        log::debug!("config: {:#?}", self.config);
        // let extractor = MetadataParser::new().await?;
        // log::debug!("extractor: {:#?}", extractor);

        if self.input.is_file() {
            self.do_split(&self.input).await?;
        } else if self.input.is_dir() {
            for entry in WalkDir::new(&self.input) {
                let entry = entry?;
                if entry.file_type().is_file() {
                    self.do_split(&entry.path()).await?;
                }
            }
        }
        Ok(true)
    }
}
