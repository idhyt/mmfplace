use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use std::collections::HashSet;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, BufReader};

lazy_static! {
    pub static ref META: MetadataReader = MetadataReader::new(None);
}

#[derive(Debug)]
pub struct MetadataReader {
    extractor_jar: PathBuf,
    xmpcore_jar: PathBuf,
}

impl MetadataReader {
    pub fn new(tool_dir: Option<&PathBuf>) -> Self {
        let tool_dir = if let Some(t) = tool_dir {
            t.to_path_buf()
        } else {
            let mut w = std::env::current_exe().unwrap();
            w.pop();
            w.join("tools")
        };
        assert!(
            tool_dir.is_dir(),
            "tool dir not found in {}",
            tool_dir.display()
        );

        let extractor_jar = tool_dir.join("metadata-extractor.jar");
        assert!(extractor_jar.is_file(), "extractor jar not found");

        let xmpcore_jar = tool_dir.join("xmpcore.jar");
        assert!(xmpcore_jar.is_file(), "xmpcore jar not found");

        // check java runtime
        let output = std::process::Command::new("java").arg("-version").output();
        assert!(output.is_ok(), "java runtime not found");
        MetadataReader {
            extractor_jar,
            xmpcore_jar,
        }
    }

    pub async fn read(&self, file_path: &PathBuf) -> Result<HashSet<String>> {
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
            file_path.to_str().unwrap(),
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
            None => return Err(anyhow!("child did not have a handle to stdout")),
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
            Ok(line) => line,
            Err(err) => {
                log::debug!("error ignore: {}", err);
                Some("".to_string())
            }
        } {
            log::debug!("{}", line);
            // only insert if len < 0xff
            if line.len() < 0xff {
                readers.insert(line);
            }
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

    #[test]
    fn test_new() {
        let meta = MetadataReader::new(None);
        println!("{:#?}", meta);
        assert!(meta.extractor_jar.is_file());
    }

    #[tokio::test]
    async fn test_read() {
        let meta = MetadataReader::new(None);
        let test = get_root().join("tests/simple.jpg");
        let readers = meta.read(&test).await.unwrap();
        println!("{:#?}", readers);
        assert!(readers.len() > 1);
        assert!(readers.contains("[Exif SubIFD] Date/Time Original = 2002:11:16 15:27:01"));
    }
}
