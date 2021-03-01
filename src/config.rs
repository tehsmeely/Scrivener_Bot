use crate::WordCloud;
use ron::de::from_reader;
use ron::ser::{to_writer_pretty, PrettyConfig};
use serde::{Deserialize, Serialize};
use serenity::prelude::TypeMapKey;
use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Duration;
use serenity::model::id::UserId;

pub struct GeneralAppConfigData;

impl TypeMapKey for GeneralAppConfigData {
    type Value = Arc<RwLock<GeneralAppConfig>>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneralAppConfig {
    pub prefix: String,
    pub wordcloud_config: Option<WordCloudConfig>,
    pub bot_admin: Option<UserId>,
}

impl Default for GeneralAppConfig {
    fn default() -> Self {
        Self {
            prefix: String::from("!"),
            wordcloud_config: Some(WordCloudConfig::default()),
            bot_admin: None,
        }
    }
}

impl GeneralAppConfig {
    pub fn load(config_file_path: &Path) -> ron::error::Result<Self> {
        match File::open(config_file_path) {
            Ok(f) => from_reader(f),
            Err(e) if e.kind() == ErrorKind::NotFound => {
                let self_ = Self::default();
                let f = File::create(config_file_path).unwrap();
                let pretty_config = PrettyConfig::new()
                    .with_depth_limit(4)
                    .with_indentor("    ".to_owned());
                to_writer_pretty(f, &self_, pretty_config).unwrap();
                Ok(self_)
            }
            Err(other) => panic!("Failed opening file: {}", other),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WordCloudConfig {
    pub python_path: PathBuf,
    pub venv_path: Option<PathBuf>,
    pub request_path: PathBuf,
    pub generated_image_path: PathBuf,
    pub timeout: Duration,
}

impl Default for WordCloudConfig {
    fn default() -> Self {
        Self {
            python_path: PathBuf::from("."),
            venv_path: Some(PathBuf::from(".")),
            request_path: PathBuf::from("in/"),
            generated_image_path: PathBuf::from("out/"),
            timeout: Duration::new(2, 0),
        }
    }
}
