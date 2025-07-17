use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{debug, error};

const EXTRACTOR: &[u8] = include_bytes!("deps/metadata-extractor-2.19.0.jar");
const XMPCORE: &[u8] = include_bytes!("deps/xmpcore-6.1.11.jar");
pub(crate) static METADATA: Lazy<MetadataReader> = Lazy::new(|| MetadataReader::new());

#[derive(Debug)]
pub struct MetadataReader {
    java: String,
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
            debug!(path = ?extractor, "Delivery the metadata-extractor");
            std::fs::write(&extractor, EXTRACTOR).expect("Failed to write metadata-extractor.jar");
        }
        if !xmpcore.is_file() {
            debug!(path = ?xmpcore, "Delivery the xmpcore.");
            std::fs::write(&xmpcore, XMPCORE).expect("Failed to write xmpcore.jar");
        }

        let java = std::env::var("MMFPLACE_JAVA").unwrap_or_else(|_| "java".to_string());
        // check java runtime
        let output = std::process::Command::new(&java).arg("-version").output();
        assert!(output.is_ok(), "java runtime not found at {}", java);

        MetadataReader {
            java,
            extractor,
            xmpcore,
        }
    }

    // #[tracing::instrument]
    pub(crate) async fn read(&self, file: &Path) -> Result<HashSet<String>> {
        let mut readers: HashSet<String> = HashSet::new();
        let class_path = format!(
            "{xc_jar}{c}{me_jar}",
            c = if cfg!(windows) { ";" } else { ":" },
            me_jar = self.extractor.display(),
            xc_jar = self.xmpcore.display()
        );
        let args = vec![
            "-Dfile.encoding=UTF-8",
            "-cp",
            &class_path,
            "com.drew.imaging.ImageMetadataReader",
            file.to_str().unwrap(),
        ];
        debug!(command=?args, "running metadata extractor.");

        let mut child = tokio::process::Command::new(&self.java)
            // .current_dir(file_path.as_ref())
            .args(args)
            // .stdin(std::process::Stdio::null())
            // .stderr(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().ok_or_else(|| {
            Error::new(
                ErrorKind::BrokenPipe,
                "child did not have a handle to stdout",
            )
        })?;

        tokio::spawn(async move {
            match child.wait().await {
                Ok(s) => debug!(status=?s, "child process exist status"),
                Err(e) => error!(error=?e, "child process encountered an error"),
            }
        });

        let mut reader = BufReader::new(stdout);
        let mut buf = Vec::new();

        // maybe error for stream did not contain valid UTF-8
        while reader.read_until(b'\n', &mut buf).await? > 0 {
            // let line = match String::from_utf8(buf.clone()) {
            //     Ok(line) => line,
            //     Err(e) => {
            //         error!(error=?e, "convert to utf-8 string error");
            //         continue;
            //     }
            // };
            let line = String::from_utf8_lossy(&buf).trim().to_string();
            debug!("{}", line);
            // 将类似  Unicode 🦀 非 ascii 使用 - 替换
            // 之所有不使用 retain(|c| c.is_ascii()) 是有可能出现在时间中间
            let line = line.replace(|c: char| !c.is_ascii(), "-");
            if line.len() < 0xff {
                readers.insert(line);
            }
            buf.clear();
        }
        Ok(readers)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn get_root() -> PathBuf {
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    #[tokio::test]
    async fn test_read() {
        println!("Metadata {:#?}", METADATA);
        assert!(METADATA.extractor.is_file());
        assert!(METADATA.xmpcore.is_file());

        let test = get_root().join("tests/2002/11/simple.jpg");
        let readers = METADATA.read(test.as_path()).await.unwrap();
        println!("{:#?}", readers);
        assert!(readers.len() > 1);
        assert!(readers.contains("[Exif SubIFD] Date/Time Original = 2002:11:16 15:27:01"));
    }
}
