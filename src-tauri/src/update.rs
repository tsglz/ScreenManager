use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub download_url: Option<String>,
    pub release_notes: Option<String>,
}

pub fn get_current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

pub fn compare_versions(current: &str, latest: &str) -> bool {
    let current_parts: Vec<&str> = current.trim_start_matches('v').split('.').collect();
    let latest_parts: Vec<&str> = latest.trim_start_matches('v').split('.').collect();

    for i in 0..3 {
        let current_part = current_parts.get(i).unwrap_or(&"0");
        let latest_part = latest_parts.get(i).unwrap_or(&"0");

        let current_num: i32 = current_part.parse().unwrap_or(0);
        let latest_num: i32 = latest_part.parse().unwrap_or(0);

        if current_num < latest_num {
            return true;
        } else if current_num > latest_num {
            return false;
        }
    }

    false
}

pub fn check_for_updates(_github_repo: &str) -> Result<UpdateInfo, String> {
    let current_version = get_current_version();

    Ok(UpdateInfo {
        has_update: false,
        current_version: current_version.clone(),
        latest_version: current_version,
        download_url: None,
        release_notes: None,
    })
}

pub fn get_app_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ScreenTime")
}

pub fn ensure_app_data_dir() -> Result<PathBuf, String> {
    let dir = get_app_data_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn get_config_path() -> PathBuf {
    get_app_data_dir().join("config.json")
}

pub fn get_categories_path() -> PathBuf {
    get_app_data_dir().join("categories.json")
}
