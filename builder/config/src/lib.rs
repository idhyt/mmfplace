use lazy_static::lazy_static;
use regex::{Error, Regex};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

lazy_static! {
    pub static ref CONFIG: Config = Config::new(None);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strptime {
    pub fmt: String,
    pub test: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parser {
    /// check the string in target or not.
    pub check: String,
    /// the regex to match the string.
    pub regex: String,
    #[serde(default = "capture_index")]
    pub index: Option<u8>,
}

fn capture_index() -> Option<u8> {
    Some(1)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Additional {
    pub name: String,
    pub dateparse: Vec<Parser>,
    pub striptimes: Vec<Strptime>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    pub batch_size: usize,
    pub striptimes: Vec<Strptime>,
    pub dateparse: Vec<Parser>,
    pub typeparse: Vec<Parser>,
    pub blacklist: Vec<String>,
    pub retain_suffix: Vec<String>,
    pub additionals: Option<Vec<Additional>>,
}

impl Config {
    /// load config.toml
    pub fn new(file: Option<PathBuf>) -> Self {
        let file = match file {
            Some(f) => f,
            None => {
                let mut work_dir =
                    std::env::current_exe().expect("failed to get current execute directory");
                work_dir.pop();
                work_dir.join("config.toml")
            }
        };
        assert!(file.is_file(), "config.toml not found");
        log::debug!("loading config from: {:?}", file);
        let content = std::fs::read_to_string(file).expect("failed to read config.toml");
        let cfg: Config = toml::from_str(&content).expect("failed to parse config.toml");
        cfg
    }
}

impl Parser {
    pub fn capture(&self, text: &str) -> Result<String, Error> {
        let re = Regex::new(&self.regex)?;
        match re.captures(text) {
            Some(caps) => match caps.get(self.index.unwrap() as usize) {
                Some(cap) => {
                    return Ok(cap.as_str().trim().to_owned());
                }
                None => {
                    return Err(Error::Syntax("capture index out of range".to_owned()));
                }
            },
            None => {
                return Err(Error::Syntax("no capture found".to_owned()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_config() {
        let config_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let config_path = config_dir.join("src").join("default.toml");
        println!("config file: {:?}", config_path);
        assert!(config_path.is_file());
        let config = Config::new(Some(config_path));
        println!("config: {:#?}", config);
        assert!(config.batch_size > 0);
        assert!(!config.dateparse.is_empty());
        assert!(!config.striptimes.is_empty());
        assert!(!config.blacklist.is_empty());
        assert!(!config.retain_suffix.is_empty());
        assert!(!config.additionals.is_some());
    }

    #[test]
    fn test_capture() {
        let config_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let config_path = config_dir.join("src").join("default.toml");
        println!("config file: {:?}", config_path);
        assert!(config_path.is_file());
        let config = Config::new(Some(config_path));
        println!("config: {:#?}", config);

        for parser in &config.dateparse {
            let text = format!("{}2024-12-20", &parser.check);
            let c = parser.capture(&text).unwrap();
            println!("text: {}, result: {:?}", text, c);
        }

        for parser in &config.typeparse {
            let text = format!("{} = .file_type", &parser.check);
            let c = parser.capture(&text).unwrap();
            println!("text: {}, result: {:?}", text, c);
        }
    }
}
