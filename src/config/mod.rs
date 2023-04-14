use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Result;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strptime {
    pub fmt: String,
    pub test: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stripe {
    pub name: String,
    pub regex: String,
    pub strptimes: Vec<Strptime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub dup_max: u32,
    #[serde(default)]
    pub retain_suffix: Vec<String>,
    #[serde(default)]
    pub stripes: Vec<Stripe>,
    #[serde(default)]
    pub blacklist: Vec<String>,
    #[serde(default)]
    pub additionals: Vec<Stripe>,
}

impl Config {
    pub fn new() -> Self {
        Config::load().expect("could not load config")
    }

    fn load() -> Result<Self> {
        if let Ok(contents) = fs::read_to_string("config.yml") {
            let config: Self = serde_yaml::from_str(&contents).expect("could not read config");
            Ok(config)
        } else {
            eprintln!("Could not find config.yml, creating template.");
            fs::write("config.yml", include_str!("default_config.yml"))?;
            Self::load()
        }
    }

    pub fn update(mut self, file_path: impl AsRef<Path>) -> Self {
        if let Ok(contents) = fs::read_to_string(&file_path) {
            let config: Self = serde_yaml::from_str(&contents).expect("could not read config");
            if config.dup_max > 0 {
                self.dup_max = config.dup_max;
            }
            if config.stripes.len() > 0 {
                self.stripes.extend(config.stripes);
            }
            if config.blacklist.len() > 0 {
                self.blacklist.extend(config.blacklist);
            }
            if config.additionals.len() > 0 {
                self.additionals.extend(config.additionals);
            }
            if config.retain_suffix.len() > 0 {
                self.retain_suffix.extend(config.retain_suffix);
            }
        }

        self
    }
}
