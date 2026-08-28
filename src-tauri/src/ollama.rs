use chrono::{Datelike, Duration as ChronoDuration, NaiveDate};
use crate::config;
use crate::database::{AppAggStat, CategoryStats, Database, DateStats, HourlyHeatmapEntry, UsageRecord};
use crate::session_aggregator::{ProjectSlice, WorkSession, aggregate_sessions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Windows 下 `localhost` 有时优先解析为 IPv6 (::1)，但 Ollama 默认只监听 127.0.0.1，
/// 因此候选地址按「IPv4 优先 + localhost 回退」顺序尝试，避免偶发连不上的问题。
const OLLAMA_BASE_CANDIDATES: &[&str] = &[
    "http://127.0.0.1:11434",
    "http://localhost:11434",
];
const TAGS_PATH: &str = "/api/tags";
const GENERATE_PATH: &str = "/api/generate";
const MODEL_NAME: &str = "qwen3:4b-fp16";

/// 生成报告的请求总超时：大模型冷启动+生成可能很慢，放宽到 5 分钟。
const GENERATE_TIMEOUT: Duration = Duration::from_secs(300);
/// 模型列表/探测类请求超时：只是一次轻量 HTTP 调用，30 秒足够。
const PROBE_TIMEOUT: Duration = Duration::from_secs(30);
/// 网络瞬态错误的重试次数（1 次首次 + 2 次重试 = 最多 3 次尝试）。
const RETRY_TIMES: u32 = 2;
/// 每次重试前的等待时间（简单线性退避）。
const RETRY_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportData {
    pub start_date: String,
    pub end_date: String,
    pub total_duration: i64,
    /// 已淘汰：仅用于旧代码兼容，真实业务数据请使用 app_stats（含出现次数）
    #[serde(default)]
    pub top_apps: Vec<(String, i64)>,
    /// 应用聚合统计（权威）：时长 + 出现次数 + 排序
    #[serde(default)]
    pub app_stats: Vec<AppAggStat>,
    pub category_stats: Vec<CategoryStats>,
    pub records: Vec<UsageRecord>,
    pub daily_stats: Vec<DateStats>,
    /// 每个应用进程名 → 分类名的映射，用于在报告里给每条记录标注真实分类
    #[serde(default)]
    pub app_categories: HashMap<String, String>,
    /// 工作会话列表（连续专注时段 + 被打断信息）
    #[serde(default)]
    pub work_sessions: Vec<WorkSession>,
    /// 小时级时间分布，用于分析一天中的效率高峰/低谷
    #[serde(default)]
    pub hourly_heatmap: Vec<HourlyHeatmapEntry>,
    /// 本次一共覆盖多少天（用于动态计算样本量和AI提示）
    #[serde(default)]
    pub day_span: i64,
    /// 每日节奏摘要（周一~周日按日期排序）：仅在日/周/月报渲染时填充；日报只有一行。
    #[serde(default)]
    pub daily_rhythm: Vec<DailyRhythmRow>,
    /// 环比上一个等长区间对比；None 表示当前区间无前序区间可用（比如区间第一天就是最早数据的日期）。
    #[serde(default)]
    pub compare_to_prev: Option<PrevRangeCompare>,
}

/// 「本周节奏」表中每一行的数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DailyRhythmRow {
    pub date: String,
    /// 星期几，中文，如 "周一"
    pub weekday: String,
    /// 当天总秒数
    pub total_seconds: i64,
    /// 与前一日的差值（正 = 多，负 = 少），第一天填 0
    pub delta_seconds: i64,
    /// 与前一日相比百分比；第一天为 None（无前序）
    pub delta_pct: Option<f64>,
    /// Top3 应用名（显示简化名）。取 `get_daily_top_apps` 的权威数据（不是 records 样本）。
    pub top_apps: Vec<String>,
    /// 当天最长一段专注会话，形如 "1h28m 09:14–10:42"；当天 0 时是空字符串
    pub longest_session: String,
}

/// 环比上一个等长区间的 5 项关键指标
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrevRangeCompare {
    pub prev_start: String,
    pub prev_end: String,
    /// 当前/上一区间总秒数
    pub cur_total: i64,
    pub prev_total: i64,
    pub cur_daily_avg: i64,
    pub prev_daily_avg: i64,
    /// 专注段数（会话数）
    pub cur_sessions: i64,
    pub prev_sessions: i64,
    /// 开发类占比（0~1）
    pub cur_dev_ratio: f64,
    pub prev_dev_ratio: f64,
    /// 分心类占比（0~1）
    pub cur_distractor_ratio: f64,
    pub prev_distractor_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateReportParams {
    pub report_type: String,
    pub start_date: String,
    pub end_date: String,
    /// 可选：本次生成报告使用的模型。None 时使用配置里的默认模型或硬编码默认模型。
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModelInfo {
    /// 模型名称（即 Ollama 里 "name" 字段，形如 "qwen3:4b-fp16"）
    pub name: String,
    /// 模型大小（人类可读格式，如 "4.2 GB"）
    pub size_label: String,
    /// 模型家族：如 "qwen3" / "llama3" / "bert"(embedding 已过滤)
    pub family: String,
    /// 参数量等级：如 "4B"
    pub parameter_size: String,
    /// 量化等级：如 "FP16" / "Q4_0"
    pub quantization: String,
    /// 修改时间（RFC 3339 字符串，若解析失败则为空）
    pub modified_at: String,
}

#[derive(Debug, Deserialize)]
struct TagsResponseModelDetails {
    family: Option<String>,
    families: Option<Vec<String>>,
    parameter_size: Option<String>,
    quantization_level: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TagsResponseModel {
    name: String,
    model: Option<String>,
    modified_at: Option<String>,
    size: Option<u64>,
    digest: Option<String>,
    details: Option<TagsResponseModelDetails>,
}

#[derive(Debug, Deserialize)]
struct TagsResponse {
    models: Vec<TagsResponseModel>,
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// 判断一个 Ollama 模型是否是 embedding 模型（需要从"生成报告用的模型列表"里排除）。
/// 识别依据（任一命中即排除）：
/// - name 包含 "embed"、"nomic-embed"、"bge-embed" 等关键词
/// - details.family == "bert"（BERT 类模型只做 embedding，不做文本生成）
/// - details.families 里包含 "bert"
fn is_embedding_model(m: &TagsResponseModel) -> bool {
    let name_lower = m.name.to_lowercase();
    if name_lower.contains("embed") || name_lower.contains("bge-") {
        return true;
    }
    if let Some(d) = &m.details {
        if matches!(d.family.as_deref(), Some("bert") | Some("nomic-bert") | Some("bge")) {
            return true;
        }
        if let Some(families) = &d.families {
            for f in families {
                let f = f.to_lowercase();
                if f == "bert" || f == "bge" {
                    return true;
                }
            }
        }
    }
    false
}

// —————————— 连接工具：候选地址 + 瞬态错误重试 + 错误诊断 ——————————

/// 构造一个只走 IPv4、强制直连不走代理的 reqwest 客户端。
/// Windows 下 IPv6 (::1) 是 Ollama 连接失败的高频坑，这里显式只解析 A 记录。
fn build_ollama_client(timeout: Duration) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(timeout)
        .connect_timeout(Duration::from_secs(5))
        .no_proxy()
        .local_address(Some(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED)))
        .build()
        .map_err(|e| format!("构造 Ollama HTTP 客户端失败: {}", e))
}

/// 判断一个 reqwest 错误是否属于「可重试的瞬态网络错误」。
/// 典型：连接被拒绝（Ollama 正在启动）、连接重置、超时、DNS 失败。
fn is_transient_error(e: &reqwest::Error) -> bool {
    if e.is_connect() || e.is_timeout() || e.is_request() {
        return true;
    }
    // 兜底：检查错误消息关键词
    let msg = e.to_string().to_lowercase();
    msg.contains("connection reset")
        || msg.contains("connection aborted")
        || msg.contains("broken pipe")
        || msg.contains("os error 10061") // WSAECONNREFUSED
        || msg.contains("os error 10053") // WSAECONNABORTED
        || msg.contains("os error 10054") // WSAECONNRESET
        || msg.contains("os error 111")    // Linux ECONNREFUSED
        || msg.contains("os error 110")    // Linux ETIMEDOUT
}

/// 把底层 reqwest 错误翻译成面向用户的诊断文案。
fn diagnostic_error_hint(e: &reqwest::Error, base: &str) -> String {
    let msg = e.to_string().to_lowercase();
    if e.is_connect() || msg.contains("connection refused") || msg.contains("10061") || msg.contains("os error 111") {
        return format!(
            "无法连接到 Ollama（{}）：连接被拒绝。\n\
            【诊断】Ollama 服务未启动或未监听 11434 端口。\n\
            【解决办法】请先启动 Ollama 桌面端，或在命令行运行：ollama serve",
            base
        );
    }
    if e.is_timeout() || msg.contains("timed out") || msg.contains("os error 110") {
        return format!(
            "无法连接到 Ollama（{}）：连接超时。\n\
            【诊断】网络栈异常或 Ollama 假死。\n\
            【解决办法】检查 Ollama 是否仍在运行，必要时重启 Ollama 服务",
            base
        );
    }
    if msg.contains("connection reset") || msg.contains("10054") {
        return format!(
            "无法连接到 Ollama（{}）：连接被重置。\n\
            【诊断】Ollama 进程在处理请求时崩溃或退出。\n\
            【解决办法】重新启动 Ollama，若频繁出现请检查系统日志或显存是否不足",
            base
        );
    }
    format!("无法连接到 Ollama（{}）：{}", base, e)
}

/// 在多个候选 base 地址上尝试请求，每个 base 做有限次重试，
/// 返回第一个成功的响应；若全部失败则返回聚合后的诊断错误。
async fn request_ollama_with_retry(
    client: &reqwest::Client,
    path: &str,
    body: Option<&serde_json::Value>,
    _timeout: Duration,
) -> Result<reqwest::Response, String> {
    // 用 path 派生 URL，body None => GET，否则 POST（当前只用到这两种）
    let mut last_err: Option<String> = None;

    for base in OLLAMA_BASE_CANDIDATES {
        let url = format!("{}{}", base, path);
        // 同一个 base 做多次重试
        for attempt in 0..=RETRY_TIMES {
            let req_result = match body {
                Some(b) => client.post(&url).json(b).send().await,
                None => client.get(&url).send().await,
            };
            match req_result {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    let hint = diagnostic_error_hint(&e, base);
                    let is_transient = is_transient_error(&e);
                    eprintln!(
                        "[Ollama] 请求 {} 失败 (base={}, attempt={}/{}): {}",
                        path,
                        base,
                        attempt + 1,
                        RETRY_TIMES + 1,
                        e
                    );
                    // 只在瞬态错误时重试当前 base；最后一次则跳出换 base
                    if is_transient && attempt < RETRY_TIMES {
                        tokio::time::sleep(RETRY_DELAY).await;
                        continue;
                    }
                    last_err = Some(hint);
                    break; // 当前 base 放弃
                }
            }
        }
    }

    // 全部候选地址都失败，给出汇总的诊断提示，明确告诉用户「Ollama 未打开」的可能
    Err(format!(
        "{}\n\n\
        ===== Ollama 连接自检清单 =====\n\
        1. 【最常见】Ollama 桌面端未启动：请从开始菜单启动 Ollama，\n\
            或命令行执行  `ollama serve`\n\
        2. 模型尚未下载：执行  `ollama pull {}`\n\
            （注意：当前默认模型为 {}）\n\
        3. 端口被占用或 Ollama 监听地址不对：\n\
            执行  `curl http://127.0.0.1:11434/api/tags`  应该返回 JSON\n\
        4. Windows 防火墙拦截：允许 Ollama 通过专用/公用网络\n\
        5. 如果你之前用了代理：本项目已强制绕过代理访问本地 Ollama，无需再在代理中配置",
        last_err.unwrap_or_else(|| "未知错误".to_string()),
        MODEL_NAME,
        MODEL_NAME
    ))
}

/// 健康检查 + 模型校验：生成报告前先确认 Ollama 可连 + 目标模型已 pull。
/// 不抛出错误的情况下返回「(探测到的模型列表, 请求所走的 base URL, 解析后的模型名)」。
pub async fn probe_ollama_before_generate(
    target_model: &str,
) -> Result<(Vec<OllamaModelInfo>, String, String), String> {
    let _ = PROBE_TIMEOUT; // 保留常量供阅读
    let client = build_ollama_client(PROBE_TIMEOUT)?;

    // 1) 调 /api/tags，成功则说明 Ollama 已打开
    let resp = request_ollama_with_retry(&client, TAGS_PATH, None, PROBE_TIMEOUT).await?;

    let status = resp.status();
    let tags_text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "Ollama 运行异常：/api/tags 返回 {}：{}",
            status, tags_text
        ));
    }
    let tags: TagsResponse = serde_json::from_str(&tags_text)
        .map_err(|e| format!("解析 Ollama /api/tags 响应失败: {} 原始响应:{}", e, tags_text))?;

    // 2) 构造过滤后的模型列表供前端使用
    let models: Vec<OllamaModelInfo> = tags
        .models
        .iter()
        .filter(|m| !is_embedding_model(m))
        .map(|m| {
            let family = m
                .details
                .as_ref()
                .and_then(|d| d.family.clone())
                .unwrap_or_else(|| "—".to_string());
            let parameter_size = m
                .details
                .as_ref()
                .and_then(|d| d.parameter_size.clone())
                .unwrap_or_else(|| "—".to_string());
            let quantization = m
                .details
                .as_ref()
                .and_then(|d| d.quantization_level.clone())
                .unwrap_or_else(|| "—".to_string());
            let size_label = m.size.map(format_size).unwrap_or_else(|| "—".to_string());
            OllamaModelInfo {
                name: m.name.clone(),
                size_label,
                family,
                parameter_size,
                quantization,
                modified_at: m.modified_at.clone().unwrap_or_default(),
            }
        })
        .collect();

    // 3) 检查目标模型是否在列表中（Ollama 返回的 name 区分大小写，做一次大小写不敏感匹配）
    let target_lc = target_model.to_lowercase();
    let matched = tags.models.iter().find(|m| m.name.to_lowercase() == target_lc);

    if matched.is_none() {
        // 没有任何模型时，提示更精准
        if tags.models.is_empty() {
            return Err(format!(
                "【Ollama 已连接】但本地没有任何模型。\n\
                请先在命令行执行：\n\
                \n\
                    ollama pull {}\n\
                \n\
                完成后再重试生成报告",
                target_model
            ));
        }
        let available = tags
            .models
            .iter()
            .map(|m| m.name.as_str())
            .collect::<Vec<_>>()
            .join("、");
        return Err(format!(
            "【Ollama 已连接】但找不到模型「{}」。\n\
            本机已 pull 的模型：{}\n\
            解决办法：\n\
              • 在命令行运行  `ollama pull {}`\n\
              • 或在「生成报告」页面右上角选择一个已存在的模型",
            target_model, available, target_model
        ));
    }

    // 4) 推断实际命中的 base（供错误文案使用，不影响后续请求——后续请求自身也会做候选重试）
    let reachable_base = OLLAMA_BASE_CANDIDATES.first().copied().unwrap_or("http://127.0.0.1:11434").to_string();
    Ok((models, reachable_base, target_model.to_string()))
}

/// 列出 Ollama 中所有**非 embedding** 的可用模型，供前端生成报告时选择。
/// 新增：多候选地址(127.0.0.1 + localhost) + 瞬态错误重试 + 精准诊断错误。
pub async fn list_ollama_models() -> Result<Vec<OllamaModelInfo>, String> {
    let client = build_ollama_client(PROBE_TIMEOUT)?;
    let resp = request_ollama_with_retry(&client, TAGS_PATH, None, PROBE_TIMEOUT).await?;

    let status = resp.status();
    let tags_text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("Ollama 返回错误 ({}): {}", status, tags_text));
    }
    let tags: TagsResponse = serde_json::from_str(&tags_text)
        .map_err(|e| format!("解析 Ollama /api/tags 响应失败: {}", e))?;

    let mut result: Vec<OllamaModelInfo> = tags
        .models
        .into_iter()
        .filter(|m| !is_embedding_model(m))
        .map(|m| {
            let family = m
                .details
                .as_ref()
                .and_then(|d| d.family.clone())
                .unwrap_or_else(|| "—".to_string());
            let parameter_size = m
                .details
                .as_ref()
                .and_then(|d| d.parameter_size.clone())
                .unwrap_or_else(|| "—".to_string());
            let quantization = m
                .details
                .as_ref()
                .and_then(|d| d.quantization_level.clone())
                .unwrap_or_else(|| "—".to_string());
            let size_label = m.size.map(format_size).unwrap_or_else(|| "—".to_string());
            OllamaModelInfo {
                name: m.name,
                size_label,
                family,
                parameter_size,
                quantization,
                modified_at: m.modified_at.unwrap_or_default(),
            }
        })
        .collect();

    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

/// 解析生成报告应该使用哪个模型：
/// 优先级 1) params.model（用户本次选择）
/// 优先级 2) AppConfig.ollama_model（保存的默认模型）
/// 兜底 3) 常量 MODEL_NAME
pub fn resolve_model(params_model: Option<&str>) -> String {
    if let Some(m) = params_model {
        let m = m.trim();
        if !m.is_empty() {
            return m.to_string();
        }
    }
    let cfg = config::load_config();
    if let Some(m) = cfg.ollama_model.as_deref() {
        let m = m.trim();
        if !m.is_empty() {
            return m.to_string();
        }
    }
    MODEL_NAME.to_string()
}

fn format_duration_hm(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{}小时{}分钟", hours, minutes)
    } else if minutes > 0 {
        format!("{}分钟", minutes)
    } else {
        format!("{}秒", seconds)
    }
}

fn format_time_brief(time_str: &str) -> String {
    if let Some(time_part) = time_str.split('T').nth(1) {
        if time_part.len() >= 5 {
            return time_part[..5].to_string();
        }
    }
    time_str.to_string()
}

/// 按显示宽度截断字符串，防止非常长的 process_name 破坏上下文对齐
fn truncate_str(s: &str, max_len: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}

/// Markdown 表格单元格转义（把 | 换成全角避免破坏表格）
fn escape_md_cell(s: &str) -> String {
    s.replace('|', "｜").replace('\n', " ").trim().to_string()
}

/// 小时级分布 → 跨天按小时累计，并按时长降序返回 (hour, seconds) 完整列表
fn aggregate_hours(entries: &[HourlyHeatmapEntry]) -> Vec<(i32, i64)> {
    use std::collections::BTreeMap;
    let mut m: BTreeMap<i32, i64> = BTreeMap::new();
    for e in entries {
        *m.entry(e.hour).or_insert(0) += e.duration_seconds;
    }
    for h in 0..24 {
        m.entry(h).or_insert(0);
    }
    let mut v: Vec<(i32, i64)> = m.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    v
}

/// 返回（高峰小时&时长, 低谷小时&时长）
fn peak_valley_hours(entries: &[HourlyHeatmapEntry]) -> ((i32, i64), (i32, i64)) {
    let agg = aggregate_hours(entries);
    let peak = agg.first().copied().unwrap_or((12, 0));
    let valley = agg.iter().rev().find(|(_, s)| *s > 0).copied()
        .unwrap_or_else(|| agg.last().copied().unwrap_or((0, 0)));
    (peak, valley)
}

// ————————————————————————————————————————————————————————————————————————
// 类型差异化 / 节奏表 / 环比 用到的纯函数小工具
// ————————————————————————————————————————————————————————————————————————

/// 把 `%Y-%m-%d` 字符串后移 `offset_days` 天，返回同格式
fn shift_date(date: &str, offset_days: i64) -> Option<String> {
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let nd = if offset_days >= 0 {
        d + ChronoDuration::days(offset_days)
    } else {
        d - ChronoDuration::days(-offset_days)
    };
    Some(nd.format("%Y-%m-%d").to_string())
}

/// 中文星期名，周一~周日
fn weekday_cn(date: &str) -> String {
    if let Ok(d) = NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        match d.weekday().number_from_monday() {
            1 => "周一".to_string(),
            2 => "周二".to_string(),
            3 => "周三".to_string(),
            4 => "周四".to_string(),
            5 => "周五".to_string(),
            6 => "周六".to_string(),
            _ => "周日".to_string(),
        }
    } else {
        "—".to_string()
    }
}

/// 分类名是否属于"开发工具类"（用于环比 + 技术报告）
fn is_dev_category(name: &str) -> bool {
    let s = name.to_lowercase();
    let kw = ["开发", "编程", "代码", "ide", "编辑器", "工具", "coding", "dev"];
    kw.iter().any(|k| s.contains(k))
}

/// 分类名是否属于"娱乐/分心源"（用于环比 + 建议区提示词）
fn is_distractor_category(name: &str) -> bool {
    let s = name;
    let kw = ["游戏", "娱乐", "社交", "视频", "资讯", "短视频", "影音", "新闻", "小说"];
    kw.iter().any(|k| s.contains(k))
}

/// 从分类统计列表里，按 key 分类累加秒数；以及总秒数
fn sum_seconds_by<'a, F>(cats: &'a [CategoryStats], pred: F) -> (i64, i64)
where
    F: Fn(&str) -> bool,
{
    let mut match_sec = 0i64;
    let mut total = 0i64;
    for c in cats {
        total += c.duration_seconds;
        if pred(&c.category_name) {
            match_sec += c.duration_seconds;
        }
    }
    (match_sec, total)
}

/// 把百分比差变成带 ± 和 pp 描述（pp = percentage point）
fn fmt_pp_diff(cur: f64, prev: f64) -> String {
    let diff = cur - prev;
    let pp = diff * 100.0;
    let arrow = if diff > 1e-9 { "↑" } else if diff < -1e-9 { "↓" } else { "▬" };
    format!("{:+.1}pp {}", pp, arrow)
}

/// 数值绝对值变化：`±xx% ↑↓`，prev=0 时输出 "—" 或 "新出现↑"
fn fmt_pct_diff(cur: i64, prev: i64) -> String {
    if prev < 0 {
        return "—".to_string();
    }
    if prev == 0 {
        return if cur > 0 {
            "+∞% ↑".to_string()
        } else {
            "—".to_string()
        };
    }
    let pct = (cur as f64 - prev as f64) / (prev as f64) * 100.0;
    let arrow = if pct > 1e-9 { "↑" } else if pct < -1e-9 { "↓" } else { "▬" };
    format!("{:+.1}% {}", pct, arrow)
}

/// 跨会话按项目聚合，返回（项目名, 秒数）按时长降序；同时返回「未标记项目」秒数（项目=其他 的累计 + work_sessions 内项目数=0 的会话秒数）
fn aggregate_projects(sessions: &[WorkSession]) -> (Vec<ProjectSlice>, i64, i64) {
    let mut agg: HashMap<String, i64> = HashMap::new();
    let mut unmarked: i64 = 0;
    let mut total: i64 = 0;
    for s in sessions {
        total += s.total_seconds;
        if s.projects.is_empty() {
            // 全部归到 main_project；如果 main_project=“其他”，算未标记
            *agg.entry(s.main_project.clone()).or_insert(0) += s.total_seconds;
            if s.main_project == "其他" {
                unmarked += s.total_seconds;
            }
            continue;
        }
        for p in &s.projects {
            *agg.entry(p.name.clone()).or_insert(0) += p.seconds;
            if p.name == "其他" {
                unmarked += p.seconds;
            }
        }
    }
    let mut v: Vec<ProjectSlice> = agg
        .into_iter()
        .map(|(name, seconds)| ProjectSlice { name, seconds })
        .collect();
    v.sort_by(|a, b| b.seconds.cmp(&a.seconds));
    (v, unmarked, total)
}

/// ————————————————————————————————————————————————————————————————————————
/// 【报告骨架预渲染】程序直接算出所有表格、排名、时段分布，输出完整 Markdown 骨架。
/// 模型只负责填两处：`{{__OVERVIEW__}}` 和 `{{__SUGGESTIONS__}}`。
/// 彻底避免小模型碰任何数字计算、排序、百分比换算。
/// ————————————————————————————————————————————————————————————————————————
fn precompute_report_blocks(
    data: &ReportData,
    report_type: &str,
    periodicity: &str,
) -> String {
    let total = data.total_duration.max(1);
    let total_minutes = (data.total_duration as f64 / 60.0).round() as i64;
    let days = data.day_span.max(1);

    let mut out = String::with_capacity(4096);

    // ——— 类型差异化：先算一份 "区块开关" 和 "Top N" 规模常量 ———
    // show_apps: 是否渲染"应用排名"
    // app_top_n: 应用 Top N 的规模
    // show_categories: 是否渲染"分类统计"
    // show_hourly: 是否渲染"时段分布"
    // session_top_n: 专注时段表的 Top N
    // show_projects: 是否渲染"项目投入"表；仅 project / standard
    // show_dev_section: tech 类型要在应用表前置 "开发工具分类明细"
    // include_rhythm_compare: 日/周/月报是否渲染节奏表+环比（日报只有节奏表一行，且无环比）
    let (app_top_n, session_top_n, show_apps, show_categories, show_hourly, show_projects, show_dev_section) =
        match report_type {
            "concise" => (5, 0, true, false, false, false, false),
            "tech" => (12, 8, true, true, true, false, true),
            "project" => (10, 8, true, true, true, true, false),
            "pomodoro" => (8, 15, false, false, false, false, false),
            _standard => (12, 8, true, true, true, true, false),
        };
    let include_rhythm = matches!(periodicity, "weekly" | "monthly")
        || (periodicity == "daily" && matches!(report_type, "standard" | "project" | "tech"));
    let include_compare = matches!(periodicity, "weekly" | "monthly");

    // ====== 0. 【模型写作参考摘要：只看这里，禁止改写】 ======
    let cat_top = data.category_stats.iter().max_by_key(|c| c.duration_seconds);
    let app_top = data.app_stats.first();
    out.push_str("======【模型写作参考摘要：2-3句话即可，禁止改写具体数字】======\n");
    out.push_str(&format!("  - 模板类型：{}，周期：{}\n", report_type, periodicity));
    out.push_str(&format!("  - 范围：{} 至 {}（{} 天）\n", data.start_date, data.end_date, days));
    out.push_str(&format!("  - 总时长：{}（{} 分钟）\n", format_duration_hm(data.total_duration), total_minutes));
    if let Some(cat) = cat_top {
        let pct = (cat.duration_seconds as f64 / total as f64) * 100.0;
        out.push_str(&format!("  - 最多分类：{} {} / {:.1}%\n", cat.category_name, format_duration_hm(cat.duration_seconds), pct));
    }
    if let Some(app) = app_top {
        let pct = (app.total_seconds as f64 / total as f64) * 100.0;
        out.push_str(&format!("  - 最多应用：{} {} / {:.1}% / {}次\n", app.process_name, format_duration_hm(app.total_seconds), pct, app.record_count));
    }
    if show_hourly && !data.hourly_heatmap.is_empty() {
        let (peak, valley) = peak_valley_hours(&data.hourly_heatmap);
        out.push_str(&format!("  - 活跃高峰：{:02}:00；低活跃：{:02}:00\n", peak.0, valley.0));
    }
    if session_top_n > 0 && !data.work_sessions.is_empty() {
        if let Some(s) = data.work_sessions.iter().max_by_key(|s| s.total_seconds) {
            out.push_str(&format!("  - 最长专注：{}（{}~{}，{} 条记录）\n",
                format_duration_hm(s.total_seconds), format_time_brief(&s.start_time),
                format_time_brief(&s.end_time), s.record_count));
        }
    }
    // 识别 Top "娱乐/分心类"应用（给建议用）
    {
        let distractors: Vec<String> = data.app_stats.iter().filter(|a| {
            let cat = data.app_categories.get(&a.process_name).map(String::as_str).unwrap_or("");
            is_distractor_category(cat)
        }).take(3).map(|a| {
            let pct = (a.total_seconds as f64 / total as f64) * 100.0;
            format!("{}({:.1}%)", a.process_name, pct)
        }).collect();
        if !distractors.is_empty() {
            out.push_str(&format!("  - 潜在分心：{}\n", distractors.join("、")));
        }
    }
    // 节奏表 / 环比要点也写进参考，让 overview 不用看大段骨架就能有结论
    if include_rhythm && !data.daily_rhythm.is_empty() {
        let max_day = data.daily_rhythm.iter().max_by_key(|d| d.total_seconds);
        let min_day = data.daily_rhythm.iter().min_by_key(|d| d.total_seconds);
        if let (Some(hi), Some(lo)) = (max_day, min_day) {
            if hi.total_seconds > 0 {
                out.push_str(&format!("  - 最忙：{}（{}）；最闲：{}（{}）\n",
                    hi.date, format_duration_hm(hi.total_seconds),
                    lo.date, format_duration_hm(lo.total_seconds)));
            }
        }
    }
    if let Some(c) = &data.compare_to_prev {
        if c.prev_total > 0 || c.cur_total > 0 {
            out.push_str(&format!("  - 环比区间：{}~{}（总时长变化 {}，会话数变化 {}）\n",
                c.prev_start, c.prev_end,
                fmt_pct_diff(c.cur_total, c.prev_total),
                fmt_pct_diff(c.cur_sessions, c.prev_sessions)));
        }
    }
    // ====== Outlook 候选清单：让模型写 outlook 只在这些范围内选，不会凭空编造 ======
    if periodicity == "daily" {
        // 日报：明日待办的动作线索（不超过 5 条，每条 2~12 字短描述）
        let mut items: Vec<String> = Vec::new();
        let (proj_slice, unmarked, total_proj) = aggregate_projects(&data.work_sessions);
        // — 未标记记录多 → 明日优先补标
        if unmarked > 0 && !proj_slice.is_empty() {
            let ratio = unmarked as f64 / total_proj.max(1) as f64;
            if ratio > 0.08 {
                items.push("补标未归属的记录".to_string());
            }
        }
        // — 有分心项 > 6% → 明日安排到固定时段
        let distractor_sec: i64 = data.app_stats.iter().filter(|a| {
            let cat = data.app_categories.get(&a.process_name).map(String::as_str).unwrap_or("");
            is_distractor_category(cat)
        }).map(|a| a.total_seconds).sum();
        if distractor_sec * 100 > total * 6 {
            items.push("把社交/娱乐集中到固定时段".to_string());
        }
        // — 最长专注太短 → 尝试一个 90 分钟番茄
        if let Some(s) = data.work_sessions.iter().max_by_key(|s| s.total_seconds) {
            if s.total_seconds < 60 * 45 {
                items.push("尝试一个 45~60 分钟长专注".to_string());
            }
        }
        // — 久坐 / 单日过忙
        if total_minutes > 6 * 60 {
            items.push("下午安排 10 分钟站立休息".to_string());
        }
        // 兜底
        if items.is_empty() {
            items.push("按节奏推进当天计划".to_string());
            items.push("完成主要任务后留 10 分钟整理".to_string());
        }
        items.truncate(4);
        out.push_str("  - 明日具体待办（outlook 候选，只在这些里选，最多挑 1-2 条）：\n");
        for (i, it) in items.iter().enumerate() {
            out.push_str(&format!("     {}. {}\n", i + 1, it));
        }
    } else {
        // 周报/月报：下周关注候选（从项目 Top、开发类缺口、分心降不下来、未标记占比高 四方面挑）
        let mut items: Vec<(String, String)> = Vec::new(); // (项目/方向名, 说明)
        let (proj_slice, unmarked, total_proj) = aggregate_projects(&data.work_sessions);
        // 1) 项目投入 Top 1-2
        for p in proj_slice.iter().take(2) {
            if p.name != "其他" && p.seconds * 20 >= total_proj {
                items.push((format!("项目「{}」", p.name), "保持或增量投入".to_string()));
            }
        }
        if unmarked > 0 && total_proj > 0 && unmarked as f64 / total_proj as f64 > 0.08 {
            items.push(("项目归属补标".to_string(), "每日结束 5 分钟整理未标记记录".to_string()));
        }
        // 2) 开发占比如果环比下滑 ≥ 5pp，或 tech 报告 ≤ 40% → 提开发专注
        if let Some(c) = &data.compare_to_prev {
            if c.cur_dev_ratio + 5.0 < c.prev_dev_ratio && c.prev_dev_ratio > 0.0 {
                items.push(("开发专注时段".to_string(), "每天留一个 90 分钟不切浏览器的连续段".to_string()));
            }
        }
        if show_dev_section {
            let (dev_sec, all_sec) = sum_seconds_by(&data.category_stats, |n| is_dev_category(n));
            if all_sec > 0 && (dev_sec as f64 / all_sec as f64) < 0.40 {
                items.push(("IDE/调试时长".to_string(), "减少无关窗口切换，把代码工作集中到上午".to_string()));
            }
        }
        // 3) 分心类占比 > 10% 或环比上升
        let (dis_sec, all_sec) = sum_seconds_by(&data.category_stats, |n| is_distractor_category(n));
        let cur_ratio = if all_sec > 0 { dis_sec as f64 / all_sec as f64 } else { 0.0 };
        if cur_ratio > 0.10 {
            items.push(("社交/娱乐分心".to_string(), "集中到晚间 1 小时，避免工作时段穿插".to_string()));
        }
        if let Some(c) = &data.compare_to_prev {
            if cur_ratio - c.prev_distractor_ratio > 0.03 {
                items.push(("分心占比上升".to_string(), "连续工作前先关闭推送/通知".to_string()));
            }
        }
        // 4) 会话数环比下降 15% 或总时长 > 40h → 提休息
        if let Some(c) = &data.compare_to_prev {
            if c.prev_sessions > 5 && (c.cur_sessions as f64) < (c.prev_sessions as f64) * 0.85 {
                items.push(("专注段数下降".to_string(), "上午/下午各排一个 25 分钟番茄钟保底".to_string()));
            }
            let total_h = (c.cur_total as f64) / 3600.0;
            if total_h > 45.0 {
                items.push(("工作强度偏高".to_string(), "周五下午留整理与恢复时间".to_string()));
            }
        }
        // 兜底
        if items.is_empty() {
            items.push(("节奏保持".to_string(), "按本周节奏推进，遇长段工作穿插 10 分钟休息".to_string()));
        }
        items.truncate(5);
        out.push_str("  - 下周关注候选（outlook 候选，只在这些里选 2-3 个写短概括，不要编新项目/数字）：\n");
        for (i, (name, desc)) in items.iter().enumerate() {
            out.push_str(&format!("     {}. {} — {}\n", i + 1, name, desc));
        }
    }
    out.push_str("================================================================\n\n");

    // ====== 1. 最终报告骨架 ======
    let type_label = match report_type {
        "standard" => "标准",
        "tech" => "技术",
        "project" => "项目",
        "concise" => "简洁",
        "pomodoro" => "番茄钟",
        _ => "工作",
    };
    let title_period = if periodicity == "daily" {
        format!("{type_label}日报 · {}", data.start_date)
    } else if periodicity == "weekly" {
        format!("{type_label}周报 · {} ~ {}", data.start_date, data.end_date)
    } else {
        format!("{type_label}月报 · {} 至 {}", data.start_date, data.end_date)
    };
    out.push_str(&format!("# {title_period}\n\n"));

    out.push_str("## 总览\n\n");
    out.push_str(&format!("- **统计周期**：{} 至 {}（共 {} 天）\n", data.start_date, data.end_date, days));
    out.push_str(&format!("- **总使用时长**：{}（{} 分钟）\n", format_duration_hm(data.total_duration), total_minutes));
    if !data.daily_stats.is_empty() {
        let daily_total: i64 = data.daily_stats.iter().map(|d| d.total_duration).sum();
        let avg = daily_total / (data.daily_stats.len() as i64).max(1);
        out.push_str(&format!("- **日均使用时长**：{}\n", format_duration_hm(avg)));
    }
    if !data.work_sessions.is_empty() {
        let longest = data.work_sessions.iter().max_by_key(|s| s.total_seconds);
        if let Some(s) = longest {
            out.push_str(&format!(
                "- **最长专注段**：{}（{} ~ {}，{} 条记录）\n",
                format_duration_hm(s.total_seconds), format_time_brief(&s.start_time),
                format_time_brief(&s.end_time), s.record_count
            ));
        }
        out.push_str(&format!("- **连续专注段数**：{} 段\n", data.work_sessions.len()));
    }
    // project 型总览里加：未标记项目占比
    if show_projects {
        let (_agg, unmarked_sec, total_sec) = aggregate_projects(&data.work_sessions);
        if total_sec > 0 {
            let pct = unmarked_sec as f64 / total_sec as f64 * 100.0;
            out.push_str(&format!("- **未标记项目占比**：{:.1}%（到「工作时间线」补手动归属可以让项目统计更准确）\n", pct));
        }
    }
    // tech 型总览里加：开发类占比
    if show_dev_section {
        let (dev_sec, _all) = sum_seconds_by(&data.category_stats, is_dev_category);
        let ratio = if total > 0 { dev_sec as f64 / total as f64 * 100.0 } else { 0.0 };
        out.push_str(&format!("- **开发类占比**：{:.1}%（{}）\n", ratio, format_duration_hm(dev_sec)));
    }
    out.push_str("\n{{__OVERVIEW__}}\n\n");

    // ====== 2. 本周节奏（按天摘要表）—— 仅 include_rhythm ======
    if include_rhythm && !data.daily_rhythm.is_empty() {
        out.push_str(&format!("## {}\n\n", if periodicity == "daily" { "当日节奏" } else { "本周节奏" }));
        out.push_str("| 日期 | 星期 | 时长 | 环比昨日 | Top 3 应用 | 最长专注 |\n");
        out.push_str("|------|------|------|----------|------------|----------|\n");
        for row in &data.daily_rhythm {
            let delta_str = if row.total_seconds == 0 {
                "—".to_string()
            } else if row.delta_pct.is_none() {
                "—".to_string()
            } else {
                let pct = row.delta_pct.unwrap();
                let arrow = if pct > 1e-9 { "↑" } else if pct < -1e-9 { "↓" } else { "▬" };
                let delta_hm = if row.delta_seconds == 0 {
                    "▬".to_string()
                } else if row.delta_seconds > 0 {
                    format!("+{}", format_duration_hm(row.delta_seconds))
                } else {
                    format!("-{}", format_duration_hm(-row.delta_seconds))
                };
                format!("{delta_hm} {pct:+.1}% {arrow}")
            };
            let apps = if row.top_apps.is_empty() {
                "—".to_string()
            } else {
                row.top_apps.join(" / ")
            };
            let longest = if row.longest_session.is_empty() { "—".to_string() } else { row.longest_session.clone() };
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                row.date, row.weekday, format_duration_hm(row.total_seconds),
                delta_str, escape_md_cell(&apps), escape_md_cell(&longest)
            ));
        }
        out.push('\n');
    }

    // ====== 3. 环比对比 —— 仅 periodicity=weekly/monthly ======
    if include_compare {
        if let Some(c) = &data.compare_to_prev {
            out.push_str(&format!("## 环比上周（对比 {} ~ {}）\n\n", c.prev_start, c.prev_end));
            out.push_str("| 指标 | 本期 | 上期 | 变化 |\n");
            out.push_str("|------|------|------|------|\n");
            out.push_str(&format!(
                "| 总使用时长 | {} | {} | {} |\n",
                format_duration_hm(c.cur_total),
                format_duration_hm(c.prev_total),
                fmt_pct_diff(c.cur_total, c.prev_total),
            ));
            out.push_str(&format!(
                "| 日均使用时长 | {} | {} | {} |\n",
                format_duration_hm(c.cur_daily_avg),
                format_duration_hm(c.prev_daily_avg),
                fmt_pct_diff(c.cur_daily_avg, c.prev_daily_avg),
            ));
            out.push_str(&format!(
                "| 专注段数 | {} 段 | {} 段 | {} |\n",
                c.cur_sessions, c.prev_sessions,
                fmt_pct_diff(c.cur_sessions, c.prev_sessions),
            ));
            out.push_str(&format!(
                "| 开发类占比 | {:.1}% | {:.1}% | {} |\n",
                c.cur_dev_ratio * 100.0,
                c.prev_dev_ratio * 100.0,
                fmt_pp_diff(c.cur_dev_ratio, c.prev_dev_ratio),
            ));
            out.push_str(&format!(
                "| 分心类占比 | {:.1}% | {:.1}% | {} |\n",
                c.cur_distractor_ratio * 100.0,
                c.prev_distractor_ratio * 100.0,
                fmt_pp_diff(c.cur_distractor_ratio, c.prev_distractor_ratio),
            ));
            out.push('\n');
        }
    }

    // ====== 4. tech 专属：开发工具分类明细 ======
    if show_dev_section && !data.category_stats.is_empty() {
        out.push_str("## 开发工具分类明细\n\n");
        // 找出开发类分类，再把 app_stats 里对应分类的应用单独列出
        let dev_cats: Vec<&CategoryStats> = data.category_stats.iter().filter(|c| is_dev_category(&c.category_name)).collect();
        if dev_cats.is_empty() {
            out.push_str("_当前分类库中暂无匹配到「开发/编程/工具」等关键字的分类。请在「设置-应用分类」中把开发相关应用归到含「开发」关键字的分类。_\n\n");
        } else {
            for cat in dev_cats {
                let pct = if total > 0 { (cat.duration_seconds as f64 / total as f64) * 100.0 } else { 0.0 };
                let mins = (cat.duration_seconds as f64 / 60.0).round() as i64;
                out.push_str(&format!(
                    "### {}：{}（{} 分钟，占总时长 {:.1}%）\n\n",
                    cat.category_name, format_duration_hm(cat.duration_seconds), mins, pct
                ));
                let apps: Vec<&AppAggStat> = data.app_stats.iter().filter(|a| {
                    let c = data.app_categories.get(&a.process_name).map(String::as_str).unwrap_or("");
                    c == cat.category_name
                }).collect();
                if apps.is_empty() {
                    out.push_str("_该分类暂无 Top 应用聚合数据_\n\n");
                } else {
                    out.push_str("| # | 应用/进程 | 总使用时长 | 占总时长 | 出现次数 |\n");
                    out.push_str("|---|-----------|-----------|---------|----------|\n");
                    for (i, a) in apps.iter().take(15).enumerate() {
                        let app_pct = if total > 0 { (a.total_seconds as f64 / total as f64) * 100.0 } else { 0.0 };
                        let mins_a = (a.total_seconds as f64 / 60.0).round() as i64;
                        out.push_str(&format!(
                            "| {} | {} | {}（{} 分钟） | {:.1}% | {} |\n",
                            i + 1, escape_md_cell(&a.process_name),
                            format_duration_hm(a.total_seconds), mins_a, app_pct, a.record_count
                        ));
                    }
                    out.push('\n');
                }
            }
        }
    }

    // ====== 5. project 专属：项目投入排名 ======
    if show_projects && !data.work_sessions.is_empty() {
        let (projects, unmarked_sec, total_sec) = aggregate_projects(&data.work_sessions);
        if !projects.is_empty() {
            out.push_str("## 项目投入排名\n\n");
            let unmarked_pct = if total_sec > 0 { unmarked_sec as f64 / total_sec as f64 * 100.0 } else { 0.0 };
            out.push_str(&format!("共识别 {} 个项目维度。未标记项目占比：**{:.1}%**（到「工作时间线」卡片里点击记录左侧的色块，手动设置项目归属可改善该数据）。\n\n",
                projects.len(), unmarked_pct));
            out.push_str("| # | 项目 | 总时长 | 占会话内 |\n");
            out.push_str("|---|------|--------|----------|\n");
            for (i, p) in projects.iter().enumerate() {
                let pct = if total_sec > 0 { p.seconds as f64 / total_sec as f64 * 100.0 } else { 0.0 };
                let mins = (p.seconds as f64 / 60.0).round() as i64;
                out.push_str(&format!(
                    "| {} | {} | {}（{} 分钟） | {:.1}% |\n",
                    i + 1, escape_md_cell(&p.name),
                    format_duration_hm(p.seconds), mins, pct,
                ));
            }
            out.push('\n');
        }
    }

    // ====== 6. 应用排名 Markdown 表格 —— show_apps ======
    if show_apps {
        out.push_str("## 应用使用排名\n\n");
        let n = std::cmp::min(data.app_stats.len(), app_top_n);
        if n > 0 {
            out.push_str("| # | 应用/进程 | 分类 | 总使用时长 | 占总时长 | 出现次数 |\n");
            out.push_str("|---|-----------|------|-----------|---------|----------|\n");
            for (i, app) in data.app_stats.iter().take(n).enumerate() {
                let cat = data.app_categories.get(&app.process_name).map(|s| s.as_str()).unwrap_or("其他");
                let mins = (app.total_seconds as f64 / 60.0).round() as i64;
                let pct = (app.total_seconds as f64 / total as f64) * 100.0;
                out.push_str(&format!(
                    "| {} | {} | {} | {}（{} 分钟） | {:.1}% | {} |\n",
                    i + 1, escape_md_cell(&app.process_name), escape_md_cell(cat),
                    format_duration_hm(app.total_seconds), mins, pct, app.record_count
                ));
            }
            out.push('\n');
        }
    }

    // ====== 7. 分类统计 Markdown 表格 —— show_categories ======
    if show_categories && !data.category_stats.is_empty() {
        out.push_str("## 分类统计\n\n");
        let mut cats: Vec<&CategoryStats> = data.category_stats.iter().collect();
        cats.sort_by(|a, b| b.duration_seconds.cmp(&a.duration_seconds));
        out.push_str("| # | 分类 | 总时长 | 占总时长 |\n");
        out.push_str("|---|------|--------|---------|\n");
        for (i, cat) in cats.iter().enumerate() {
            let pct = (cat.duration_seconds as f64 / total as f64) * 100.0;
            let mins = (cat.duration_seconds as f64 / 60.0).round() as i64;
            out.push_str(&format!(
                "| {} | {} | {}（{} 分钟） | {:.1}% |\n",
                i + 1, escape_md_cell(&cat.category_name),
                format_duration_hm(cat.duration_seconds), mins, pct
            ));
        }
        out.push('\n');
    }

    // ====== 8. 时段分布 —— show_hourly ======
    if show_hourly && !data.hourly_heatmap.is_empty() {
        out.push_str("## 时段活动分布\n\n");
        let per_hour = aggregate_hours(&data.hourly_heatmap);
        let top5: Vec<String> = per_hour.iter().take(5).map(|(h, s)| {
            format!("{:02}:00（{}）", h, format_duration_hm(*s))
        }).collect();
        out.push_str(&format!("- **活跃时段 TOP 5**：{}\n", top5.join("、")));
        let low: Vec<String> = per_hour.iter().rev()
            .filter(|(_, s)| *s > 0).take(5)
            .map(|(h, s)| format!("{:02}:00（{}）", h, format_duration_hm(*s)))
            .collect();
        if !low.is_empty() {
            out.push_str(&format!("- **低活跃时段**：{}\n", low.join("、")));
        }
        out.push('\n');
    }

    // ====== 9. 专注时段（工作会话）—— session_top_n > 0 ======
    if session_top_n > 0 && !data.work_sessions.is_empty() {
        out.push_str("## 专注时段（工作会话）\n\n");
        let mut sorted: Vec<&WorkSession> = data.work_sessions.iter().collect();
        sorted.sort_by(|a, b| b.total_seconds.cmp(&a.total_seconds));
        let take_n = std::cmp::min(sorted.len(), session_top_n);
        out.push_str(&format!("共识别到 {} 段连续工作时段，时长最长的 {} 段：\n\n", sorted.len(), take_n));
        out.push_str("| # | 时段 | 时长 | 记录数 | 主要项目/应用 |\n");
        out.push_str("|---|------|------|--------|--------------|\n");
        for (i, sess) in sorted.iter().take(take_n).enumerate() {
            let proj = if sess.projects.is_empty() {
                sess.main_project.clone()
            } else {
                sess.projects.iter().take(3).map(|p| p.name.clone()).collect::<Vec<_>>().join("、")
            };
            let proj = if proj.is_empty() { "—".to_string() } else { truncate_str(&proj, 40) };
            let mins = (sess.total_seconds as f64 / 60.0).round() as i64;
            out.push_str(&format!(
                "| {} | {} ~ {} | {}（{} 分钟） | {} | {} |\n",
                i + 1, format_time_brief(&sess.start_time), format_time_brief(&sess.end_time),
                format_duration_hm(sess.total_seconds), mins, sess.record_count, escape_md_cell(&proj)
            ));
        }
        out.push('\n');
    }

    // ====== 10. 效率分析与建议（占位符） ======
    out.push_str("## 效率分析与建议\n\n");
    out.push_str("{{__SUGGESTIONS__}}\n\n");

    // ====== 11. 下周 / 明日 / 下月 关注（第 3 个模型占位符） ======
    // * 程序已在【模型写作参考摘要】末尾列好 4-5 条候选，模型只在该范围内选 1-3 条写短概括
    // * 即便是"弱模型（4b）" 也不会乱编项目名 / 数字；最坏情况它漏写了我们有兜底 fallback（见 P2-3）
    let outlook_title = if periodicity == "daily" {
        "明日关注"
    } else if periodicity == "monthly" {
        "下月调节点"
    } else {
        "下周关注"
    };
    out.push_str(&format!("## {}\n\n", outlook_title));
    out.push_str("{{__OUTLOOK__}}\n\n");

    // 末尾说明（模型不要删）
    out.push_str("---\n*报告表格、排名、百分比、次数、时长均由程序基于 usage_records 原始记录自动统计生成。*\n");

    out
}

fn build_prompt(report_type: &str, context: &str) -> String {
    // ——— 按 periodicity 分支措辞；context 里有 "周期：daily/weekly/monthly" 行，直接 contains 判断
    let (is_daily, is_monthly) = (
        context.contains("周期：daily"),
        context.contains("周期：monthly"),
    );
    let periodicity_hint = if is_daily {
        "这是日报：A 段不要出现「本周/下周」措辞，聚焦单日；C 段只写明日 1-2 个具体动作，不扯一周以上规划。"
    } else if is_monthly {
        "这是月报：A 段突出本月整体趋势、与上月对比要点；C 段写下个月要调整的 1-2 个习惯或工作节奏。"
    } else {
        "这是周报：A 段要兼顾节奏——最忙/最闲的日子、环比上周的总体方向；C 段要结合下方「下周关注候选」线索，写 1 句概括 + 最多 2 句具体动作，不泛谈。"
    };
    let type_hint = match report_type {
        "tech" => "这是技术报告：若骨架里给出了开发类占比/开发工具明细，则 B 段建议尽量围绕 IDE 连续调试时长、浏览器查资料分心源、代码编辑窗口切换频率等技术场景给具体建议。",
        "project" => "这是项目报告：B 段建议优先围绕项目投入不均、未标记项目占比偏高的现象给 1-2 条具体动作，不要空话。",
        "concise" => "这是简洁型报告：A 段 ≤ 80 字；B 段建议不超过 2 条；C 段写 1 句话就行。",
        "pomodoro" => "这是番茄钟报告：B 段聚焦「最长专注段表现」「专注段数是否达标」「>25 分钟段占比」，围绕专注间隔/休息给具体建议。",
        _ => "",
    };
    let outlook_field_note = if is_daily {
        "  \"outlook\": \"【C段：明日安排/关注的 1-2 件具体事；40 字内；确实没的可写就写“明日无特殊关注”】\","
    } else {
        "  \"outlook\": \"【C段：结合报告尾部已给出的“下周关注候选”清单，写 1 句概括性总结 + 最多 2 句具体动作；不要编造未列出的项目或数字】\","
    };
    format!(
r#"你的任务是**只写三段中文文字**填入报告骨架，不计算、不改写、不重排任何数字或表格。
{periodicity_hint}
{type_hint}

【严格操作规则（违反=报告作废）】
1. 输出必须是严格 JSON，只输出下面 JSON 模板里给出的 3 个字段，不要输出任何其它字符、标题、分隔线、解释语、Markdown。
2. 所有数字、百分比、次数、排名、表格、Markdown 标题、分类名、项目名等已由程序预写死在下方骨架中，你绝对不能再输出对它们的改写、换算或重排。
3. 不允许在 3 段文字中编造任何预计算摘要里不存在的数字（如「平均运行时长」「运行频次」「典型时长」等）。
4. 禁止输出骨架里不存在的 section（如「异常进程」「进程记录」「访问网页清单」等），也禁止自己编表格或列表。

【任务内容】
- A. overview（{{{{{{__OVERVIEW__}}}}}}）：2-4 句中文，总结花在哪里、最主要 1-2 个分类、最主要应用、最突出特征（高峰/最长专注/最忙最闲日/环比变化）。只参考【模型写作参考摘要】已给出的结论，不要自行分析。建议 80-150 字。
- B. suggestions（{{{{{{__SUGGESTIONS__}}}}}}）：2-4 条中文建议，用「1.  2.  3. 」这样的编号格式（每条 1-2 句）。可围绕：
  1) 专注分析：最长专注时段表现 / 段数达标情况
  2) 分心分析：若候选里给出了分心应用/分类，建议安排到固定时段；否则不提
  3) 效率高峰：把重要工作安排到效率高峰时段
  4) 工作强度 / 久坐 / 切换过频提醒
  5) 结合每日节奏或上周环比差异给出作息建议
  不要空话套话，每条要有具体动作。
- C. outlook（{{{{{{__OUTLOOK__}}}}}}）：1-3 句短中文，下周关注 / 明日关注 / 下月调节点。
  * 若骨架里程序已列出「下周关注候选」项目清单或「明日具体待办」，严格只在这些候选范围内给概括，不要编新的项目名或新的数字；
  * 若候选清单为空，写一句通用的「下周继续保持当前节奏，遇到长时间工作安排 10 分钟休息」类的具体动作即可；
  * 建议长度：日报 40 字内；周/月报 50-100 字。

【参考数据】
{context}

【固定输出格式（JSON，必须完全一致）】
{{{{{{
  "overview": "【你写的A段，中文】",
  "suggestions": "【你写的B段，用1. 2. 3. 这样的编号格式】",
{outlook_field_note}
}}}}}}
"#)
}


fn days_between(a: &str, b: &str) -> i64 {
    let da = NaiveDate::parse_from_str(a, "%Y-%m-%d").ok();
    let db = NaiveDate::parse_from_str(b, "%Y-%m-%d").ok();
    match (da, db) {
        (Some(a), Some(b)) => (b - a).num_days(),
        _ => 0,
    }
}

fn periodicity_and_title(report_type: &str, start: &str, end: &str) -> (&'static str, String) {
    let days = days_between(start, end) + 1;
    let periodicity = if days <= 1 {
        "daily"
    } else if days <= 31 {
        "weekly"
    } else {
        "monthly"
    };
    let type_name = match report_type {
        "standard" => "标准",
        "tech" => "技术",
        "project" => "项目",
        "concise" => "简洁",
        "pomodoro" => "番茄钟",
        _ => "工作",
    };
    let title = if periodicity == "daily" {
        format!("{}日报 · {}", type_name, start)
    } else if periodicity == "weekly" {
        format!("{}周报 · {} ~ {}", type_name, start, end)
    } else {
        format!("{}月报 · {}", type_name, &start[..7])
    };
    (periodicity, title)
}

/// 容错解析模型返回的 JSON：`{ "overview": "...", "suggestions": "..." }`
/// 解析失败时（小模型可能输出非 JSON 文字）做多层兜底：
///   1) 提取第一个 `{...}` 块再试；
/// 解析模型返回的 JSON 文本，返回 (overview, suggestions, outlook) 三段中文。
/// 3 层兜底策略：
///   1) 直接 parse；字段缺失用默认
///   2) 截取第一个 { ... } 块 parse；
///   3) 全失败返回空字符串 / 中文兜底（outlook 给通用一句）。
fn parse_model_json_output(resp: &str) -> (String, String, String) {
    #[derive(Deserialize)]
    struct Partials {
        overview: Option<String>,
        suggestions: Option<String>,
        #[serde(default)]
        outlook: Option<String>,
    }

    let trimmed = resp.trim();
    // 尝试 1：直接解析整个返回
    if let Ok(v) = serde_json::from_str::<Partials>(trimmed) {
        return (
            v.overview.unwrap_or_default().trim().to_string(),
            v.suggestions.unwrap_or_default().trim().to_string(),
            v.outlook.unwrap_or_default().trim().to_string(),
        );
    }
    // 尝试 2：截取第一个 { ... } 块
    if let (Some(open), Some(close)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if open < close {
            if let Ok(v) = serde_json::from_str::<Partials>(&trimmed[open..=close]) {
                return (
                    v.overview.unwrap_or_default().trim().to_string(),
                    v.suggestions.unwrap_or_default().trim().to_string(),
                    v.outlook.unwrap_or_default().trim().to_string(),
                );
            }
        }
    }
    // 兜底：把返回前 6 行当 overview，suggestions 用默认；outlook 给通用一句
    let fallback_overview = if trimmed.is_empty() {
        "（本次生成未返回总览描述）".to_string()
    } else {
        trimmed.lines().take(6).collect::<Vec<_>>().join(" ").trim().to_string()
    };
    let fallback_suggestions = "1. 将重要工作安排在效率高峰时段，连续专注超过 90 分钟后主动安排 10 分钟休息。\n\
2. 若使用过程中出现游戏、社交、视频等分心源，可集中安排在每日固定时段以减少对专注的打断。\n\
3. 根据每日时长分布，把同类任务安排在同一天的连续时段，减少任务切换带来的认知成本。".to_string();
    let fallback_outlook = "继续保持当前节奏，遇到连续长段工作穿插 10 分钟休息；未标记的记录在收尾阶段补标即可。".to_string();
    (fallback_overview, fallback_suggestions, fallback_outlook)
}

pub async fn generate_and_save_report(
    db: &Arc<Mutex<Database>>,
    params: GenerateReportParams,
) -> Result<(i64, String), String> {
    let model_name = resolve_model(params.model.as_deref());

    // ——— Step 0（前置检查）：探测 Ollama 是否可连 + 目标模型是否已 pull ———
    // 这步避免用户等到 30~120s 超时后才知道"Ollama 没打开"或"模型没拉"
    let _probe_info = probe_ollama_before_generate(&model_name).await
        .map_err(|e| format!("[前置检查失败] {}\n\n提示：先解决上述问题，再点生成按钮", e))?;

    // periodicity 在后面骨架渲染 / 提示词分支都要用，先算
    let periodicity = periodicity_and_title(&params.report_type, &params.start_date, &params.end_date).0.to_string();

    let data = {
        let db_guard = db.lock().map_err(|e| format!("DB lock error: {}", e))?;

        // 计算区间天数：动态决定 records 样本量 & app_stats 条数
        let day_span = days_between(&params.start_date, &params.end_date).max(1);
        let records_limit = if day_span <= 1 {
            120
        } else if day_span <= 7 {
            300
        } else {
            500
        };
        let app_stats_limit = 30usize;

        let total = db_guard.get_range_total(&params.start_date, &params.end_date)
            .map_err(|e| format!("Query total error: {}", e))?;
        let app_stats = db_guard.get_range_app_stats(&params.start_date, &params.end_date, app_stats_limit)
            .map_err(|e| format!("Query app stats error: {}", e))?;
        // 兼容：从 app_stats 派生出简单的 top_apps
        let top_apps: Vec<(String, i64)> = app_stats
            .iter()
            .map(|a| (a.process_name.clone(), a.total_seconds))
            .collect();
        let category_stats = db_guard.get_range_category_stats(&params.start_date, &params.end_date)
            .map_err(|e| format!("Query category stats error: {}", e))?;
        let records = db_guard.get_records_for_date_range(&params.start_date, &params.end_date, records_limit)
            .map_err(|e| format!("Query records error: {}", e))?;
        let daily_stats = db_guard.get_date_range_stats(&params.start_date, &params.end_date)
            .map_err(|e| format!("Query daily stats error: {}", e))?;

        if total == 0 {
            return Err("所选时间范围内没有工作数据".to_string());
        }

        // 构建进程名 → 分类映射：
        // 从「全量 app_stats (Top30) ∪ records 样本」两个集合的并集查分类，
        // 保证最常使用的应用分类标注 100% 正确，不会因为 records 样本不足而被标成"其他"。
        let mut app_categories: HashMap<String, String> = HashMap::new();
        let mut insert_cat = |name: &str| {
            if !app_categories.contains_key(name) {
                if let Ok(cat) = db_guard.get_category_for_app(name) {
                    app_categories.insert(name.to_string(), cat);
                }
            }
        };
        for app in &app_stats {
            insert_cat(&app.process_name);
        }
        for record in &records {
            insert_cat(&record.process_name);
        }

        // 工作会话：连续专注时段与被打断信息
        let work_sessions = {
            let idle_threshold: i64 = db_guard
                .get_config_value("session_idle_threshold")
                .and_then(|v| v.parse().ok())
                .unwrap_or(900);
            let switch_threshold: i64 = db_guard
                .get_config_value("session_switch_threshold")
                .and_then(|v| v.parse().ok())
                .unwrap_or(300);
            let (ordered_records, overrides) = (
                db_guard.get_records_for_range_ordered(&params.start_date, &params.end_date).unwrap_or_default(),
                db_guard.get_overrides_for_range(&params.start_date, &params.end_date).unwrap_or_default(),
            );
            aggregate_sessions(ordered_records, overrides, idle_threshold, switch_threshold)
        };

        // 小时级时间分布：分析效率高峰/低谷
        let hourly_heatmap = db_guard
            .get_hourly_heatmap_for_range(&params.start_date, &params.end_date)
            .unwrap_or_default();

        // ——— 【P0-b】每日节奏表：日期 + Top3 应用 + 最长专注 + 环比昨日 ———
        // 1) 用 daily_stats 确定日期范围（已按日期升序）
        // 2) 循环每天调 get_daily_top_apps(date, 3) → 权威 Top3 应用
        // 3) 会话按天划分取最长专注
        // 4) 程序内算 delta 与百分比
        let mut daily_rhythm: Vec<DailyRhythmRow> = Vec::with_capacity(daily_stats.len());
        {
            // 会话按日期（start_time 前 10 位）分组取最长
            let mut longest_by_day: HashMap<String, &WorkSession> = HashMap::new();
            for s in &work_sessions {
                let d = if s.start_time.len() >= 10 { &s.start_time[..10] } else { continue };
                match longest_by_day.get(d).copied() {
                    None => { longest_by_day.insert(d.to_string(), s); }
                    Some(cur) if s.total_seconds > cur.total_seconds => { longest_by_day.insert(d.to_string(), s); }
                    _ => {}
                }
            }
            let mut prev_sec: Option<i64> = None;
            for day in &daily_stats {
                let top3 = db_guard.get_daily_top_apps(&day.date, 3)
                    .unwrap_or_default();
                let top_apps: Vec<String> = top3.iter().map(|(name, _)| {
                    // 去掉 .exe 后缀，使表格简洁
                    let base = name.trim();
                    if base.len() > 4 {
                        let tail = &base[base.len()-4..];
                        if tail.eq_ignore_ascii_case(".exe") {
                            base[..(base.len()-4)].to_string()
                        } else {
                            base.to_string()
                        }
                    } else {
                        base.to_string()
                    }
                }).collect();
                let longest_session = longest_by_day.get(&day.date).map(|s| {
                    format!("{} {}~{}", format_duration_hm(s.total_seconds), format_time_brief(&s.start_time), format_time_brief(&s.end_time))
                }).unwrap_or_default();
                let (delta_sec, delta_pct) = match prev_sec {
                    None => (0, None),
                    Some(prev) => {
                        let delta = day.total_duration - prev;
                        let pct = if prev <= 0 {
                            if day.total_duration > 0 { Some(100.0) } else { Some(0.0) }
                        } else {
                            Some((day.total_duration as f64 - prev as f64) / prev as f64 * 100.0)
                        };
                        (delta, pct)
                    }
                };
                prev_sec = Some(day.total_duration);
                daily_rhythm.push(DailyRhythmRow {
                    date: day.date.clone(),
                    weekday: weekday_cn(&day.date),
                    total_seconds: day.total_duration,
                    delta_seconds: delta_sec,
                    delta_pct,
                    top_apps,
                    longest_session,
                });
            }
        }

        // ——— 【P1】环比上周：取上一个等长区间的 5 项指标 ———
        // 仅对 weekly / monthly 做；等长区间 = 当前区间 [start, end] 向前平移 day_span 天
        let compare_to_prev: Option<PrevRangeCompare> = if periodicity == "daily" {
            None
        } else {
            let (Some(prev_start), Some(prev_end)) = (
                shift_date(&params.start_date, -day_span),
                shift_date(&params.end_date, -day_span),
            ) else {
                return Err("日期解析失败，无法构造环比区间".to_string());
            };
            if prev_end < prev_start {
                None
            } else {
                // 实际查询
                let prev_total = db_guard.get_range_total(&prev_start, &prev_end).unwrap_or(0);
                let prev_daily_stats = db_guard.get_date_range_stats(&prev_start, &prev_end).unwrap_or_default();
                let prev_category_stats = db_guard.get_range_category_stats(&prev_start, &prev_end).unwrap_or_default();
                // prev 会话数：用 ordered_records + overrides 再聚合一次
                let prev_sessions: Vec<WorkSession> = {
                    let idle_threshold: i64 = db_guard
                        .get_config_value("session_idle_threshold")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(900);
                    let switch_threshold: i64 = db_guard
                        .get_config_value("session_switch_threshold")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(300);
                    let (ordered, overrides) = (
                        db_guard.get_records_for_range_ordered(&prev_start, &prev_end).unwrap_or_default(),
                        db_guard.get_overrides_for_range(&prev_start, &prev_end).unwrap_or_default(),
                    );
                    aggregate_sessions(ordered, overrides, idle_threshold, switch_threshold)
                };
                // 当前值 & 计算
                let cur_total = total;
                let cur_days = daily_stats.len().max(1) as i64;
                let prev_days = prev_daily_stats.len().max(1) as i64;
                let cur_daily_avg = cur_total / cur_days.max(1);
                let prev_daily_avg = prev_total / prev_days.max(1);
                let cur_sessions = work_sessions.len() as i64;
                let prev_sessions_cnt = prev_sessions.len() as i64;

                // 当前分类比率
                let (cur_dev_sec, all_cur_sec) = sum_seconds_by(&category_stats, is_dev_category);
                let (cur_dis_sec, _) = sum_seconds_by(&category_stats, is_distractor_category);
                let cur_dev_ratio = if all_cur_sec > 0 { cur_dev_sec as f64 / all_cur_sec as f64 } else { 0.0 };
                let cur_distractor_ratio = if all_cur_sec > 0 { cur_dis_sec as f64 / all_cur_sec as f64 } else { 0.0 };
                // prev 分类比率
                let (prev_dev_sec, all_prev_sec) = sum_seconds_by(&prev_category_stats, is_dev_category);
                let (prev_dis_sec, _) = sum_seconds_by(&prev_category_stats, is_distractor_category);
                let prev_dev_ratio = if all_prev_sec > 0 { prev_dev_sec as f64 / all_prev_sec as f64 } else { 0.0 };
                let prev_distractor_ratio = if all_prev_sec > 0 { prev_dis_sec as f64 / all_prev_sec as f64 } else { 0.0 };

                Some(PrevRangeCompare {
                    prev_start, prev_end,
                    cur_total, prev_total,
                    cur_daily_avg, prev_daily_avg,
                    cur_sessions, prev_sessions: prev_sessions_cnt,
                    cur_dev_ratio, prev_dev_ratio,
                    cur_distractor_ratio, prev_distractor_ratio,
                })
            }
        };

        ReportData {
            start_date: params.start_date.clone(),
            end_date: params.end_date.clone(),
            total_duration: total,
            top_apps,
            app_stats,
            category_stats,
            records,
            daily_stats,
            app_categories,
            work_sessions,
            hourly_heatmap,
            day_span,
            daily_rhythm,
            compare_to_prev,
        }
    };

    let context = precompute_report_blocks(&data, &params.report_type, &periodicity);
    let prompt = build_prompt(&params.report_type, &context);

    // ——— Step 1：构建长时间请求客户端，用 IPv4 直连 + 5 分钟总超时 ———
    let client = build_ollama_client(GENERATE_TIMEOUT)?;

    // 降低 temperature：模型只需要做简单的中文措辞润色，不需要发散
    // num_predict 从 600 → 800：支持 outlook 第 3 段；三段落合计 ~450-650 字中文，800 token 安全
    let body = serde_json::json!({
        "model": model_name,
        "prompt": prompt,
        "stream": false,
        "options": {
            "temperature": 0.35,
            "num_predict": 800
        }
    });

    // ——— Step 2：用候选地址(127.0.0.1 + localhost) + 瞬态重试发送请求 ———
    let response = request_ollama_with_retry(&client, GENERATE_PATH, Some(&body), GENERATE_TIMEOUT).await?;

    // ——— Step 3：处理业务级错误（404 模型不存在、500 服务异常等）———
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        let text_lc = text.to_lowercase();
        let extra_hint = if status.as_u16() == 404
            || text_lc.contains("not found")
            || text_lc.contains("model") && text_lc.contains("not")
        {
            format!(
                "\n【诊断】模型 {} 未 pull 或名称不正确。\n\
                可用模型查看：生成报告页面右上角，或命令行  `ollama list`",
                model_name
            )
        } else if status.is_server_error() {
            "\n【诊断】Ollama 服务端在生成时报错（可能显存不足、模型损坏等）。\n\
              【解决办法】在命令行执行一次  `ollama run {}`  看是否正常，若失败请  `ollama pull {}`  重新拉取"
                .replace("{}", &model_name)
        } else {
            String::new()
        };
        return Err(format!("Ollama 返回错误 ({}): {}{}", status, text, extra_hint));
    }

    let ollama_resp: OllamaResponse = response
        .json()
        .await
        .map_err(|e| format!("解析 Ollama 响应失败（返回的内容不是有效JSON）：{}\n\
            可能原因：Ollama 进程在生成中途崩溃，或大模型冷启动超时", e))?;

    // ——— Step 4：解析模型返回的 JSON → 取 overview / suggestions / outlook → 与程序预渲染骨架合并 ———
    let (overview, suggestions, outlook) = parse_model_json_output(&ollama_resp.response);
    // outlook 若真的为空（模型漏写 + fallback 意外为空），给最后一道保险
    let outlook = if outlook.trim().is_empty() {
        "继续保持当前节奏；未标记的记录在每日收尾花 5 分钟补标。".to_string()
    } else {
        outlook
    };
    // 最终合并：把三处占位符替换为模型的文字（overview / suggestions / outlook）
    // 注意：context 里头部有「模型写作参考摘要」段（用 ====== 分隔），仅供模型在 prompt 里参考，
    // 不能出现在最终报告正文中。这里在替换前先把参考摘要剥离掉。
    let skeleton = {
        let marker = "================================================================";
        if let Some(idx) = context.find(marker) {
            context[idx + marker.len()..].trim_start().to_string()
        } else {
            context.clone()
        }
    };
    let content_md = skeleton
        .replacen("{{__OVERVIEW__}}", &overview, 1)
        .replacen("{{__SUGGESTIONS__}}", &suggestions, 1)
        .replacen("{{__OUTLOOK__}}", &outlook, 1);

    let (periodicity, title) = periodicity_and_title(&params.report_type, &params.start_date, &params.end_date);

    let report_id = {
        let db_guard = db.lock().map_err(|e| format!("DB lock error: {}", e))?;
        db_guard
            .insert_or_update_report(&params.report_type, periodicity, &params.start_date, &params.end_date, &title, &content_md)
            .map_err(|e| format!("保存报告失败: {}", e))?
    };

    Ok((report_id, content_md))
}
