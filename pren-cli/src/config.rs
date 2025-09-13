use pren_core::file_storage::FileStorage;
use std::path::PathBuf;

pub fn initialize_storage(base_path: Option<String>) -> FileStorage {
    match base_path {
        Some(path) => FileStorage {
            base_path: PathBuf::from(path),
        },
        None => {
            let path = option_env!("PREN_STORAGE_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(FileStorage::default().base_path));
            FileStorage { base_path: path }
        }
    }
}
