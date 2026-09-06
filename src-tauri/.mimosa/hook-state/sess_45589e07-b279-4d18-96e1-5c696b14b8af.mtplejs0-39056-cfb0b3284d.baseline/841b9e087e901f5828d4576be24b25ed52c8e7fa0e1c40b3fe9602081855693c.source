use std::path::PathBuf;

pub fn get_current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

pub fn get_app_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ScreenTime")
}

pub fn _ensure_app_data_dir() -> Result<PathBuf, String> {
    let dir = get_app_data_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn _get_config_path() -> PathBuf {
    get_app_data_dir().join("config.json")
}

pub fn _get_categories_path() -> PathBuf {
    get_app_data_dir().join("categories.json")
}
