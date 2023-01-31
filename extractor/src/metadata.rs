use anyhow::{anyhow, Result};
use async_process::{Command, Stdio};
use futures_lite::{io::BufReader, prelude::*};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct MetadataReader {
    extractor_jar: PathBuf,
    xmpcore_jar: PathBuf,
}

impl MetadataReader {
    pub async fn new<T>(work_dir: Option<T>) -> Result<Self>
    where
        T: AsRef<Path>,
    {
        let jlibs_dir = match work_dir {
            Some(wd) => {
                let check_path = wd.as_ref().clone().join("extractor").join("jlibs");
                if check_path.is_dir() {
                    check_path
                } else {
                    Path::new("extractor").join("jlibs")
                }
            }
            None => Path::new("extractor").join("jlibs"),
        };

        if !jlibs_dir.is_dir() {
            return Err(anyhow!(
                "java libs directory not found in {}",
                jlibs_dir.display()
            ));
        }

        let extractor_jar = jlibs_dir.clone().join("metadata-extractor-2.18.0.jar");
        if !extractor_jar.is_file() {
            return Err(anyhow!(
                "metadata-extractor file not found in {}",
                extractor_jar.display()
            ));
        }

        let xmpcore_jar = jlibs_dir.clone().join("xmpcore-6.1.11.jar");
        if !xmpcore_jar.is_file() {
            return Err(anyhow!(
                "xmpcore file not found in {}",
                xmpcore_jar.display()
            ));
        }

        Ok(MetadataReader {
            extractor_jar,
            xmpcore_jar,
        })
    }

    pub async fn read(&self, file_path: impl AsRef<Path>) -> Result<HashSet<String>> {
        let mut readers: HashSet<String> = HashSet::new();
        let cps = if cfg!(windows) { ";" } else { ":" };
        let mut child = Command::new("java")
            .arg("-cp")
            .arg(format!(
                "{xc_jar}{c}{me_jar}",
                c = cps,
                me_jar = self.extractor_jar.display(),
                xc_jar = self.xmpcore_jar.display()
            ))
            .arg("com.drew.imaging.ImageMetadataReader")
            .arg(file_path.as_ref().as_os_str())
            .stdout(Stdio::piped())
            // .stderr(Stdio::piped())
            .spawn()?;

        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        while let Some(line) = lines.next().await {
            // let check_str = line.unwrap().clone();
            // log::debug!("{}", check_str);
            match line {
                Ok(l) => {
                    if l.len() < 0xff {
                        readers.insert(l);
                    }
                }
                Err(e) => {
                    log::error!("read line error: {}", e);
                }
            }
        }

        Ok(readers)
    }
}
