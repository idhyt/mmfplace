use once_cell::sync::Lazy;
use regex::{Error, Regex};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub static CONFIG: Lazy<Config> = Lazy::new(|| Config::new());
const CONFIG_DEFAULT: &str = include_str!("default.toml");

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
    pub fn new() -> Self {
        let config = {
            let mut work_dir =
                std::env::current_exe().expect("failed to get current execute directory");
            work_dir.pop();
            work_dir.join("config.toml")
        };
        if !config.is_file() {
            std::fs::write(&config, CONFIG_DEFAULT).expect("Failed to write config.toml");
            log::warn!(
                "🚨 The first run creates a default config file at {}",
                config.display()
            )
        }
        Self::load_from_file(config.as_path())
    }

    pub fn load_from_file(f: &Path) -> Self {
        log::debug!("Loading config from: {}", f.display());
        let content = std::fs::read_to_string(f).expect("Failed to read config.toml");
        let cfg: Config = toml::from_str(&content).expect("Failed to parse config.toml");
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

    #[test]
    fn test_config() {
        assert!(CONFIG.batch_size > 0);
        assert!(!CONFIG.dateparse.is_empty());
        assert!(!CONFIG.striptimes.is_empty());
        assert!(!CONFIG.blacklist.is_empty());
        assert!(!CONFIG.retain_suffix.is_empty());
        assert!(CONFIG.additionals.is_some());
    }

    #[test]
    fn test_capture() {
        println!("config: {:#?}", CONFIG);

        for parser in &CONFIG.dateparse {
            let text = format!("{}2024-12-20", &parser.check);
            let c = parser.capture(&text).unwrap();
            println!("text: {}, result: {:?}", text, c);
        }

        for parser in &CONFIG.typeparse {
            let text = format!("{} = .file_type", &parser.check);
            let c = parser.capture(&text).unwrap();
            println!("text: {}, result: {:?}", text, c);
        }
    }
}
