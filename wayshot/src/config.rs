use crate::utils::EncodingFormat;
use jpegxl_rs::encode::EncoderSpeed;
use serde::{Deserialize, Serialize};
use std::{env, io::Read, path::PathBuf};
use tracing::Level;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub base: Option<Base>,
    pub file: Option<File>,
    pub encoding: Option<Encoding>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            base: Some(Base::default()),
            file: Some(File::default()),
            encoding: Some(Encoding::default()),
        }
    }
}

impl Config {
    pub fn load(path: &PathBuf) -> Option<Config> {
        let mut config_file = std::fs::File::open(path).ok()?;
        let mut config_str = String::new();
        config_file.read_to_string(&mut config_str).ok()?;

        toml::from_str(&config_str).ok()?
    }

    pub fn get_default_path() -> PathBuf {
        dirs::config_local_dir()
            .map(|path| path.join("wayshot").join("config.toml"))
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Base {
    pub output: Option<String>,
    pub cursor: Option<bool>,
    pub clipboard: Option<bool>,
    pub file: Option<bool>,
    pub stdout: Option<bool>,
    pub log_level: Option<String>,
    pub notifications: Option<bool>,
}

impl Default for Base {
    fn default() -> Self {
        Base {
            output: None,
            cursor: Some(false),
            clipboard: Some(false),
            file: Some(true),
            stdout: Some(false),
            log_level: Some("info".to_string()),
            notifications: Some(true),
        }
    }
}

impl Base {
    pub fn get_log_level(&self) -> Level {
        self.log_level
            .as_ref()
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct File {
    pub path: Option<PathBuf>,
    pub name_format: Option<String>,
    pub encoding: Option<EncodingFormat>,
}

impl Default for File {
    fn default() -> Self {
        File {
            path: Some(File::get_default_screenshot_dir()),
            name_format: Some("wayshot-%Y_%m_%d-%H_%M_%S".to_string()),
            encoding: Some(EncodingFormat::Png),
        }
    }
}

impl File {
    pub fn get_default_screenshot_dir() -> PathBuf {
        dirs::picture_dir()
            .map(|path| path.join("Screenshots"))
            .unwrap_or_else(|| env::current_dir().unwrap_or_default())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Encoding {
    pub jxl: Option<Jxl>,
    pub png: Option<Png>,
}

impl Default for Encoding {
    fn default() -> Self {
        Encoding {
            jxl: Some(Jxl::default()),
            png: Some(Png::default()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Jxl {
    pub lossless: Option<bool>,
    pub distance: Option<f32>,
    pub effort: Option<u8>,
}

impl Default for Jxl {
    fn default() -> Self {
        Jxl {
            lossless: Some(false),
            distance: Some(1.0),
            effort: Some(7),
        }
    }
}

impl Jxl {
    pub fn get_lossless(&self) -> bool {
        self.lossless.unwrap_or(false)
    }

    pub fn get_distance(&self) -> f32 {
        self.distance.unwrap_or(1.0)
    }

    pub fn get_effort(&self) -> u8 {
        self.effort.unwrap_or(7)
    }

    pub fn get_encoder_speed(&self) -> EncoderSpeed {
        match self.get_effort() {
            1 => EncoderSpeed::Lightning,
            2 => EncoderSpeed::Thunder,
            3 => EncoderSpeed::Falcon,
            4 => EncoderSpeed::Cheetah,
            5 => EncoderSpeed::Hare,
            6 => EncoderSpeed::Wombat,
            7 => EncoderSpeed::Squirrel,
            8 => EncoderSpeed::Kitten,
            9 => EncoderSpeed::Tortoise,
            10 => EncoderSpeed::Glacier,
            _ => EncoderSpeed::Squirrel,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PngCompression {
    Level(u8),
    Named(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Png {
    pub compression: Option<PngCompression>,
    pub filter: Option<String>,
}

impl Default for Png {
    fn default() -> Self {
        Png {
            compression: Some(PngCompression::Named("default".to_string())),
            filter: Some("adaptive".to_string()),
        }
    }
}

impl Png {
    pub fn get_compression(&self) -> image::codecs::png::CompressionType {
        match self
            .compression
            .as_ref()
            .unwrap_or(&PngCompression::Named("default".to_string()))
        {
            PngCompression::Level(level) => {
                if *level <= 9 {
                    image::codecs::png::CompressionType::Level(*level)
                } else {
                    image::codecs::png::CompressionType::Default
                }
            }
            PngCompression::Named(name) => match name.as_str() {
                "default" => image::codecs::png::CompressionType::Default,
                "best" => image::codecs::png::CompressionType::Best,
                "fast" => image::codecs::png::CompressionType::Fast,
                "none" | "uncompressed" => image::codecs::png::CompressionType::Uncompressed,
                _ => image::codecs::png::CompressionType::Default,
            },
        }
    }

    pub fn get_filter(&self) -> image::codecs::png::FilterType {
        match self.filter.as_deref().unwrap_or("default") {
            "none" => image::codecs::png::FilterType::NoFilter,
            "sub" => image::codecs::png::FilterType::Sub,
            "up" => image::codecs::png::FilterType::Up,
            "avg" => image::codecs::png::FilterType::Avg,
            "paeth" => image::codecs::png::FilterType::Paeth,
            "adaptive" => image::codecs::png::FilterType::Adaptive,
            _ => image::codecs::png::FilterType::Adaptive,
        }
    }
}
