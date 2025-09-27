use crate::constants::PREN_CLI;
use confy::ConfyError;
use pren_core::file_storage::FileStorage;
use serde::{Deserialize, Serialize};
use std::env::home_dir;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct PrenCliConfig {
    pub base_path: String,
    pub(crate) model_config: ModelConfig,
}

#[derive(Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_name: String,
    pub api_key: String,
    pub base_url: String,
}


impl Default for PrenCliConfig {
    fn default() -> Self {
        let base_path = home_dir()
            .map(|p| p.join("pren").join("prompts"))
            .unwrap_or_else(|| PathBuf::from("pren/prompts"));

        Self {
            base_path: base_path.display().to_string(),
            model_config: ModelConfig::default(),
        }
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model_name: String::from("qwen/qwen3-30b-a3b-2507"),
            api_key: String::from(""), // TODO: We should be getting this from env, this is just temporary
            base_url: String::from("http://192.168.0.20:1234/v1"),
        }
    }
}

pub fn get_storage() -> FileStorage {
    let config: Result<PrenCliConfig, ConfyError> = confy::load(PREN_CLI, None);
    match config {
        Ok(config) => FileStorage {
            base_path: PathBuf::from(config.base_path),
        },
        _ => {
            eprintln!("Error: Problem loading config. Exiting...",);
            std::process::exit(exitcode::CONFIG);
        }
    }
}
