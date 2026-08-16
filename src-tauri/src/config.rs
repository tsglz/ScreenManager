use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

use crate::update::get_app_data_dir;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    /// HTTP/HTTPS 代理，例如 "http://127.0.0.1:7890"。
    /// 为 None 或空字符串时表示不设置进程级代理。
    #[serde(default)]
    pub http_proxy: Option<String>,
}

pub fn get_config_path() -> PathBuf {
    get_app_data_dir().join("config.json")
}

fn ensure_data_dir() -> Result<(), String> {
    let dir = get_app_data_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("创建数据目录失败: {}", e))
}

pub fn load_config() -> AppConfig {
    let path = get_config_path();
    if !path.exists() {
        return AppConfig::default();
    }
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            eprintln!(
                "[Config] 解析 {} 失败 ({}), 使用默认配置",
                path.display(),
                e
            );
            AppConfig::default()
        }),
        Err(e) => {
            eprintln!("[Config] 读取 {} 失败 ({}), 使用默认配置", path.display(), e);
            AppConfig::default()
        }
    }
}

pub fn save_config(cfg: &AppConfig) -> Result<(), String> {
    ensure_data_dir()?;
    let path = get_config_path();
    let json = serde_json::to_string_pretty(cfg).map_err(|e| format!("序列化配置失败: {}", e))?;
    fs::write(&path, json).map_err(|e| format!("写入配置 {} 失败: {}", path.display(), e))?;
    Ok(())
}

/// 将配置中的代理应用到进程环境变量（HTTP_PROXY / HTTPS_PROXY）。
/// 由于 reqwest / plugin-updater 均会在运行时读取这些变量，
/// 必须在执行任何网络请求之前（main 函数入口早期）调用一次。
pub fn apply_proxy_env(cfg: &AppConfig) {
    let proxy = cfg.http_proxy.as_deref().unwrap_or("").trim();
    if proxy.is_empty() {
        return;
    }
    env::set_var("HTTP_PROXY", proxy);
    env::set_var("HTTPS_PROXY", proxy);
    // 部分库（含部分 reqwest 配置）读的是全小写版本
    env::set_var("http_proxy", proxy);
    env::set_var("https_proxy", proxy);
    eprintln!("[Config] 已应用进程代理: {}", proxy);
}

/// 运行时切换代理并立即生效（设置页面点保存时调用）。
pub fn set_proxy_at_runtime(proxy: Option<String>) {
    let p = proxy.as_deref().unwrap_or("").trim().to_string();
    if p.is_empty() {
        env::remove_var("HTTP_PROXY");
        env::remove_var("HTTPS_PROXY");
        env::remove_var("http_proxy");
        env::remove_var("https_proxy");
        eprintln!("[Config] 已清除进程代理");
    } else {
        env::set_var("HTTP_PROXY", &p);
        env::set_var("HTTPS_PROXY", &p);
        env::set_var("http_proxy", &p);
        env::set_var("https_proxy", &p);
        eprintln!("[Config] 已实时应用代理: {}", p);
    }
}
