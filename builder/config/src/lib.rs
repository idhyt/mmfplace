use lazy_static::lazy_static;
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
pub struct Stripe {
    pub name: String,
    pub regex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Additional {
    pub name: String,
    pub regex: String,
    pub dateparse: Vec<Strptime>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    pub batch_size: u32,
    pub dateparse: Vec<Strptime>,
    pub stripes: Vec<Stripe>,
    pub blacklist: Vec<String>,
    pub retain_suffix: Vec<String>,
    pub additionals: Vec<Additional>,
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
        assert!(!config.stripes.is_empty());
        assert!(!config.blacklist.is_empty());
        assert!(!config.retain_suffix.is_empty());
        assert!(!config.additionals.is_empty());
    }
}
