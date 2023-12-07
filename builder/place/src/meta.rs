use anyhow::{anyhow, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct MetadataReader {
    extractor_jar: PathBuf,
    xmpcore_jar: PathBuf,
}

impl MetadataReader {
    pub async fn new<T>(tool_dir: T) -> Result<Self>
    where
        T: AsRef<Path>,
    {
        let jlibs_dir = tool_dir.as_ref();
        let extractor_jar = jlibs_dir.join("metadata-extractor-2.18.0.jar");
        if !extractor_jar.is_file() {
            return Err(anyhow!(
                "metadata-extractor file not found in {:?}",
                extractor_jar
            ));
        }

        let xmpcore_jar = jlibs_dir.join("xmpcore-6.1.11.jar");
        if !xmpcore_jar.is_file() {
            return Err(anyhow!("xmpcore file not found in {:?}", xmpcore_jar));
        }

        // check java runtime
        match Command::new("java")
            .arg("-version")
            .stderr(Stdio::piped())
            .output()
            .await
        {
            Ok(output) => match output.status.success() {
                true => log::debug!(
                    "java runtime found: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
                false => {
                    return Err(anyhow!(
                        "java runtime not found, please install java runtime first."
                    ));
                }
            },
            Err(err) => {
                return Err(anyhow!(
                    "java runtime not found, please install java runtime first, {:?}",
                    err
                ));
            }
        }

        Ok(MetadataReader {
            extractor_jar,
            xmpcore_jar,
        })
    }

    pub async fn read(&self, file_path: impl AsRef<Path>) -> Result<HashSet<String>> {
        let mut readers: HashSet<String> = HashSet::new();
        let class_path = format!(
            "{xc_jar}{c}{me_jar}",
            c = if cfg!(windows) { ";" } else { ":" },
            me_jar = self.extractor_jar.display(),
            xc_jar = self.xmpcore_jar.display()
        );
        let args = vec![
            "-cp",
            &class_path,
            "com.drew.imaging.ImageMetadataReader",
            file_path.as_ref().as_os_str().to_str().unwrap(),
        ];
        log::debug!("run command args: {}", args.join(" "));
        let mut child = Command::new("java")
            // .current_dir(file_path.as_ref())
            .args(args)
            .stdout(Stdio::piped())
            .spawn()?;

        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => return Err(anyhow!("child did not have a handle to stdout")),
        };

        let mut reader = BufReader::new(stdout).lines();

        tokio::spawn(async move {
            match child.wait().await {
                Ok(status) => log::debug!("child status was: {}", status),
                Err(err) => log::error!("child process encountered an error: {}", err),
            }
        });

        // while let Some(line) = reader.next_line().await? {
        //     log::debug!("{}", line);
        //     if line.len() < 0xff {
        //         readers.insert(line);
        //     }
        // }

        // stream did not contain valid UTF-8
        while let Some(line) = match reader.next_line().await {
            Ok(line) => line,
            Err(err) => {
                log::debug!("error ignore: {}", err);
                Some("".to_string())
            }
        } {
            log::debug!("{}", line);
            if line.len() < 0xff {
                readers.insert(line);
            }
        }
        Ok(readers)
    }
}
