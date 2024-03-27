use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, path::PathBuf};
use toml;

use crate::utils::EncodingFormat;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub screenshot: Option<Screenshot>,
    pub fs: Option<Fs>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            screenshot: Some(Screenshot::default()),
            fs: Some(Fs::default()),
        }
    }
}

impl Config {
    pub fn load(path: &PathBuf) -> Option<Config> {
        let mut config_file = File::open(path).ok()?;
        let mut config_str = String::new();
        config_file.read_to_string(&mut config_str).ok()?;

        toml::from_str(&mut config_str).ok()?
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Screenshot {
    pub display: Option<String>,
    pub cursor: Option<bool>,
    pub clipboard: Option<bool>,
    pub fs: Option<bool>,
    pub stdout: Option<bool>,
}

impl Default for Screenshot {
    fn default() -> Self {
        Screenshot {
            display: None,
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
            path: None,
            format: Some("wayshot-%Y_%m_%d-%H_%M_%S".to_string()),
            encoding: Some(EncodingFormat::Png),
        }
    }
}
