use futures::lock::Mutex;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Parser {
    pub dup_max: u32,
    pub batch_size: u32,
    pub retain_suffix: Vec<String>,
    pub stripes: Vec<Stripe>,
    pub blacklist: Vec<String>,
    pub additionals: Vec<Stripe>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    pub work_dir: PathBuf,
    pub tools_dir: PathBuf,
    pub output: PathBuf,
    pub parser: Parser,
}

impl Config {
    pub fn new() -> Self {
        Config::default()
    }

    /// set the work directory
    pub fn set_work_dir(&mut self, work_dir: Option<PathBuf>) {
        let work_dir = match work_dir {
            Some(path) => path,
            None => {
                let mut work_dir = std::env::current_exe().expect("failed to get current dir");
                work_dir.pop();
                work_dir
            }
        };
        // self.work_dir = work_dir.canonicalize().unwrap();
        self.work_dir = work_dir;
        self.tools_dir = self.work_dir.join("tools");
    }

    /// set the output directory
    pub fn set_output_dir(&mut self, output: Option<PathBuf>) {
        self.output = match output {
            Some(path) => path,
            None => {
                let mut output = self.work_dir.clone();
                output.push("output");
                output
            }
        };
        if !self.output.is_dir() {
            std::fs::create_dir_all(&self.output).expect("failed to create output directory");
        }
    }

    fn get_cfile(&self, config: Option<PathBuf>) -> PathBuf {
        let p = if let Some(path) = config {
            path
        } else {
            let config = self.work_dir.join("config.yaml");
            if !config.is_file() {
                std::fs::write(&config, include_str!("default.yaml"))
                    .expect("failed to write config.yaml");
            }
            config
        };
        p
    }

    /// load config
    pub fn load(&mut self, config: Option<PathBuf>) {
        let cf = self.get_cfile(config);
        let content = std::fs::read_to_string(cf).expect("failed to read config.yaml");
        let cfg: Parser = serde_yaml::from_str(&content).expect("failed to parse config.yaml");
        self.parser = cfg
    }

    // pub fn update(mut self, file_path: impl AsRef<Path>) -> Self {
    //     if let Ok(contents) = std::fs::read_to_string(&file_path) {
    //         let config: Self = serde_yaml::from_str(&contents).expect("could not read config");
    //         if config.dup_max > 0 {
    //             self.dup_max = config.dup_max;
    //         }
    //         if config.stripes.len() > 0 {
    //             self.stripes.extend(config.stripes);
    //         }
    //         if config.blacklist.len() > 0 {
    //             self.blacklist.extend(config.blacklist);
    //         }
    //         if config.additionals.len() > 0 {
    //             self.additionals.extend(config.additionals);
    //         }
    //         if config.retain_suffix.len() > 0 {
    //             self.retain_suffix.extend(config.retain_suffix);
    //         }
    //     }

    //     self
    // }
}

lazy_static! {
    pub static ref CONFIG: Mutex<Config> = Mutex::new(Config::new());
    // pub static ref CONFIG: Arc<Mutex<Config>> = Arc::new(Mutex::new(Config::new()));
}
