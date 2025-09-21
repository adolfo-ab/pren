use crate::constants::PREN_CLI;
use confy::ConfyError;
use pren_core::file_storage::FileStorage;
use serde::{Deserialize, Serialize};
use std::env::home_dir;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct PrenCliConfig {
    base_path: String,
}

impl Default for PrenCliConfig {
    fn default() -> Self {
        let base_path = home_dir()
            .map(|p| p.join("pren").join("prompts"))
            .unwrap_or_else(|| PathBuf::from("pren").join("prompts"));

        Self {
            base_path: String::from(base_path.to_str().unwrap()),
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
