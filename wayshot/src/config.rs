use crate::utils::EncodingFormat;
use serde::{Deserialize, Serialize};
use std::{env, fs::File, io::Read, path::PathBuf};
use tracing::Level;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub screenshot: Option<Screenshot>,
    pub fs: Option<Fs>,
    pub log: Option<Log>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            screenshot: Some(Screenshot::default()),
            fs: Some(Fs::default()),
            log: Some(Log::default()),
        }
    }
}

impl Config {
    pub fn load(path: &PathBuf) -> Option<Config> {
        let mut config_file = File::open(path).ok()?;
        let mut config_str = String::new();
        config_file.read_to_string(&mut config_str).ok()?;

        toml::from_str(&config_str).ok()?
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Screenshot {
    pub output: Option<String>,
    pub cursor: Option<bool>,
    pub clipboard: Option<bool>,
    pub fs: Option<bool>,
    pub stdout: Option<bool>,
}

impl Default for Screenshot {
    fn default() -> Self {
        Screenshot {
            output: None,
            cursor: Some(false),
            clipboard: Some(true),
            fs: Some(true),
            stdout: Some(false),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fs {
    pub path: Option<PathBuf>,
    pub format: Option<String>,
    pub encoding: Option<EncodingFormat>,
}

impl Default for Fs {
    fn default() -> Self {
        Fs {
            path: Some(env::current_dir().unwrap_or_default()),
            format: Some("wayshot-%Y_%m_%d-%H_%M_%S".to_string()),
            encoding: Some(EncodingFormat::Png),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Log {
    pub level: Option<String>,
}

impl Default for Log {
    fn default() -> Self {
        Log {
            level: Some("info".to_string()),
        }
    }
}

impl Log {
    pub fn get_level(self) -> Level {
        self.level
            .map_or(Level::INFO, |level| match level.as_str() {
                "trace" => Level::TRACE,
                "debug" => Level::DEBUG,
                "info" => Level::INFO,
                "warn" => Level::WARN,
                "error" => Level::ERROR,
                _ => Level::INFO,
            })
    }
}
