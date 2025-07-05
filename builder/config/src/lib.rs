use once_cell::sync::Lazy;
use regex::{Error, Regex};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub static CONFIG: Lazy<Config> = Lazy::new(|| Config::new());
const CONFIG_DEFAULT: &str = include_str!("default.toml");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripTime {
    pub fmt: String,
    pub test: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Capture {
    /// check the string in target or not.
    pub check: String,
    /// the regex to match the string.
    #[serde(deserialize_with = "deserialize_regex")]
    pub regex: Regex,
    #[serde(default = "capture_index")]
    pub index: Option<u8>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DateParse {
    pub ignore: Option<Vec<String>>,
    pub list: Vec<StripTime>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct DateRegex {
    pub ignore: Option<Vec<String>>,
    pub list: Vec<Capture>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct TypeRegex {
    pub ignore: Option<Vec<String>>,
    pub list: Vec<Capture>,
}

fn deserialize_regex<'de, D>(deserializer: D) -> Result<Regex, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Regex::new(&s).map_err(serde::de::Error::custom)
}

fn capture_index() -> Option<u8> {
    Some(1)
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Config {
    pub batch_size: usize,
    pub dateparse: DateParse,
    pub dateregex: DateRegex,
    pub typeregex: TypeRegex,
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

impl Capture {
    pub fn capture(&self, text: &str) -> Result<String, Error> {
        // let re = Regex::new(&self.regex)?;
        let re = &self.regex;
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
        assert!(!CONFIG.dateparse.list.is_empty());
        assert!(!CONFIG.dateregex.list.is_empty());
        assert!(CONFIG.dateregex.ignore.is_some());
        assert!(!CONFIG.typeregex.list.is_empty());
        assert!(CONFIG.typeregex.ignore.is_some());
    }

    #[test]
    fn test_date_regex() {
        println!("config: {:#?}", CONFIG);

        for capture in &CONFIG.dateregex.list {
            let text = format!("{}2024-12-20", &capture.check);
            let c = capture.capture(&text).unwrap();
            println!("text: {}, result: {:?}", text, c);
        }

        for capture in &CONFIG.typeregex.list {
            let text = format!("{} = .file_type", &capture.check);
            let c = capture.capture(&text).unwrap();
            println!("text: {}, result: {:?}", text, c);
        }
    }
}
