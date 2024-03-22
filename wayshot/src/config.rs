use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, path::PathBuf};
use toml;

use crate::utils::EncodingFormat;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub screenshot: Option<Screenshot>,
    #[serde(rename = "clipboard")]
    pub clipboard: Option<Clipboard>,
    #[serde(rename = "filesystem")]
    pub filesystem: Option<Filesystem>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            screenshot: Some(Screenshot::default()),
            clipboard: Some(Clipboard::default()),
            filesystem: Some(Filesystem::default()),
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
}

impl Default for Screenshot {
    fn default() -> Self {
        Screenshot {
            display: None,
            cursor: Some(false),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Clipboard {
    pub clipboard: Option<bool>,
}

impl Default for Clipboard {
    fn default() -> Self {
        Clipboard {
            clipboard: Some(true),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Filesystem {
    pub filesystem: Option<bool>,
    pub path: Option<PathBuf>,
    pub format: Option<String>,
    pub encoding: Option<EncodingFormat>,
}

impl Default for Filesystem {
    fn default() -> Self {
        Filesystem {
            filesystem: Some(true),
            path: None,
            // PR #93
            format: Some("wayshot-%Y_%m_%d-%H_%M_%S".to_string()),
            encoding: Some(EncodingFormat::Png),
        }
    }
}
