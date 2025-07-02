use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, BufReader};

const EXTRACTOR: &[u8] = include_bytes!("deps/metadata-extractor-2.19.0.jar");
const XMPCORE: &[u8] = include_bytes!("deps/xmpcore-6.1.11.jar");
pub static METADATA: Lazy<MetadataReader> = Lazy::new(|| MetadataReader::new());

#[derive(Debug)]
pub struct MetadataReader {
    extractor: PathBuf,
    xmpcore: PathBuf,
}

impl MetadataReader {
    fn new() -> Self {
        let tools = {
            let mut work_dir =
                std::env::current_exe().expect("failed to get current execute directory");
            work_dir.pop();
            work_dir.join("tools")
        };
        if !tools.is_dir() {
            std::fs::create_dir_all(&tools).unwrap();
        }
        // free tools
        let (extractor, xmpcore) = (
            tools.join("metadata-extractor-2.19.0.jar"),
            tools.join("xmpcore-6.1.11.jar"),
        );
        if !extractor.is_file() {
            log::debug!("Delivery the metadata-extractor {}", extractor.display());
            std::fs::write(&extractor, EXTRACTOR).expect("Failed to write metadata-extractor.jar");
        }
        if !xmpcore.is_file() {
            log::debug!("Delivery the xmpcore {}", xmpcore.display());
            std::fs::write(&xmpcore, XMPCORE).expect("Failed to write xmpcore.jar");
        }

        // check java runtime
        let output = std::process::Command::new("java").arg("-version").output();
        assert!(output.is_ok(), "java runtime not found");

        MetadataReader { extractor, xmpcore }
    }

    pub async fn read(&self, file: &Path) -> Result<HashSet<String>> {
        let mut readers: HashSet<String> = HashSet::new();
        let class_path = format!(
            "{xc_jar}{c}{me_jar}",
            c = if cfg!(windows) { ";" } else { ":" },
            me_jar = self.extractor.display(),
            xc_jar = self.xmpcore.display()
        );
        let args = vec![
            "-cp",
            &class_path,
            "com.drew.imaging.ImageMetadataReader",
            file.to_str().unwrap(),
        ];

        log::debug!("run command args: {:?}", args);

        let mut child = tokio::process::Command::new("java")
            // .current_dir(file_path.as_ref())
            .args(args)
            .stderr(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                return Err(Error::new(
                    ErrorKind::BrokenPipe,
                    "child did not have a handle to stdout",
                ));
            }
        };

        let mut reader = BufReader::new(stdout).lines();

        tokio::spawn(async move {
            match child.wait().await {
                Ok(status) => log::debug!("child status was: {}", status),
                Err(err) => log::error!("child process encountered an error: {}", err),
            }
        });

        // stream did not contain valid UTF-8
        while let Some(line) = match reader.next_line().await {
            Ok(line) => Some(line),
            Err(err) => {
                log::debug!("error ignore: {}", err);
                None
            }
        } {
            if let Some(l) = line {
                log::debug!("{}", l);
                if l.len() < 0xff {
                    readers.insert(l);
                }
            }
        }
        Ok(readers)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_read() {
        println!("Metadata {:#?}", METADATA);
        assert!(METADATA.extractor.is_file());
        assert!(METADATA.xmpcore.is_file());

        let test = PathBuf::from("tests/simple.jpg");
        let readers = METADATA.read(test.as_path()).await.unwrap();
        println!("{:#?}", readers);
        assert!(readers.len() > 1);
        assert!(readers.contains("[Exif SubIFD] Date/Time Original = 2002:11:16 15:27:01"));
    }
}
