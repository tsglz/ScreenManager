use chrono::NaiveDate;
use crate::config;
use crate::database::{AppAggStat, CategoryStats, Database, DateStats, HourlyHeatmapEntry, UsageRecord};
use crate::session_aggregator::{WorkSession, aggregate_sessions};
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

/// ————————————————————————————————————————————————————————————————————————
/// 【报告骨架预渲染】程序直接算出所有表格、排名、时段分布，输出完整 Markdown 骨架。
/// 模型只负责填两处：`{{__OVERVIEW__}}` 和 `{{__SUGGESTIONS__}}`。
/// 彻底避免小模型碰任何数字计算、排序、百分比换算。
/// ————————————————————————————————————————————————————————————————————————
fn precompute_report_blocks(data: &ReportData) -> String {
    let total = data.total_duration.max(1);
    let total_minutes = (data.total_duration as f64 / 60.0).round() as i64;
    let days = data.day_span.max(1);

    let mut out = String::with_capacity(4096);

    // ====== 0. 【模型写作参考摘要：只看这里，禁止改写】 ======
    let cat_top = data.category_stats.iter().max_by_key(|c| c.duration_seconds);
    let app_top = data.app_stats.first();
    out.push_str("======【模型写作参考摘要：2-3句话即可，禁止改写具体数字】======\n");
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
    if !data.hourly_heatmap.is_empty() {
        let (peak, valley) = peak_valley_hours(&data.hourly_heatmap);
        out.push_str(&format!("  - 活跃高峰：{:02}:00；低活跃：{:02}:00\n", peak.0, valley.0));
    }
    if !data.work_sessions.is_empty() {
        if let Some(s) = data.work_sessions.iter().max_by_key(|s| s.total_seconds) {
            out.push_str(&format!("  - 最长专注：{}（{}~{}，{} 条记录）\n",
                format_duration_hm(s.total_seconds), format_time_brief(&s.start_time),
                format_time_brief(&s.end_time), s.record_count));
        }
    }
    // 识别 Top "娱乐/分心类"应用（给建议用）：分类含"游戏""娱乐""社交""资讯"关键词
    {
        let distractors: Vec<String> = data.app_stats.iter().filter(|a| {
            let cat = data.app_categories.get(&a.process_name).map(String::as_str).unwrap_or("");
            cat.contains("游戏") || cat.contains("娱乐") || cat.contains("社交") || cat.contains("资讯") || cat.contains("视频")
        }).take(3).map(|a| {
            let pct = (a.total_seconds as f64 / total as f64) * 100.0;
            format!("{}({:.1}%)", a.process_name, pct)
        }).collect();
        if !distractors.is_empty() {
            out.push_str(&format!("  - 潜在分心：{}\n", distractors.join("、")));
        }
    }
    out.push_str("================================================================\n\n");

    // ====== 1. 最终报告骨架：所有表格/排名/百分比/次数 都预写死 ======
    let title_period = if days <= 1 {
        format!("日报 · {}", data.start_date)
    } else if days <= 7 {
        format!("周报 · {} ~ {}", data.start_date, data.end_date)
    } else {
        format!("月报 · {} 至 {}", data.start_date, data.end_date)
    };
    out.push_str(&format!("# {title_period} 使用情况报告\n\n"));

    out.push_str("## 总览\n\n");
    out.push_str(&format!("- **统计周期**：{} 至 {}（共 {} 天）\n", data.start_date, data.end_date, days));
    out.push_str(&format!("- **总使用时长**：{}（{} 分钟）\n", format_duration_hm(data.total_duration), total_minutes));
    if !data.daily_stats.is_empty() {
        let daily_total: i64 = data.daily_stats.iter().map(|d| d.total_duration).sum();
        let avg = daily_total / (data.daily_stats.len() as i64).max(1);
        out.push_str(&format!("- **日均使用时长**：{}\n", format_duration_hm(avg)));
        // 每日时长列表（含占比）
        out.push_str("- **每日时长分布**：\n");
        for d in &data.daily_stats {
            let pct = (d.total_duration as f64 / total as f64) * 100.0;
            out.push_str(&format!("  - {}：{}（{:.1}%）\n", d.date, format_duration_hm(d.total_duration), pct));
        }
    }
    out.push_str("\n{{__OVERVIEW__}}\n\n");

    // ====== 2. 应用排名 Markdown 表格 ======
    out.push_str("## 应用使用排名\n\n");
    let top_n = std::cmp::min(data.app_stats.len(), 12);
    if top_n > 0 {
        out.push_str("| # | 应用/进程 | 分类 | 总使用时长 | 占总时长 | 出现次数 |\n");
        out.push_str("|---|-----------|------|-----------|---------|----------|\n");
        for (i, app) in data.app_stats.iter().take(top_n).enumerate() {
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

    // ====== 3. 分类统计 Markdown 表格 ======
    out.push_str("## 分类统计\n\n");
    if !data.category_stats.is_empty() {
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

    // ====== 4. 时段分布 ======
    if !data.hourly_heatmap.is_empty() {
        out.push_str("## 时段活动分布\n\n");
        let per_hour = aggregate_hours(&data.hourly_heatmap);
        let top5: Vec<String> = per_hour.iter().take(5).map(|(h, s)| {
            format!("{:02}:00（{}）", h, format_duration_hm(*s))
        }).collect();
        out.push_str(&format!("- **活跃时段 TOP 5**：{}\n", top5.join("、")));
        // 低谷：输出 > 0 的最不活跃的几个
        let low: Vec<String> = per_hour.iter().rev()
            .filter(|(_, s)| *s > 0).take(5)
            .map(|(h, s)| format!("{:02}:00（{}）", h, format_duration_hm(*s)))
            .collect();
        if !low.is_empty() {
            out.push_str(&format!("- **低活跃时段**：{}\n", low.join("、")));
        }
        out.push('\n');
    }

    // ====== 5. 专注时段（工作会话）======
    if !data.work_sessions.is_empty() {
        out.push_str("## 专注时段（工作会话）\n\n");
        let mut sorted: Vec<&WorkSession> = data.work_sessions.iter().collect();
        sorted.sort_by(|a, b| b.total_seconds.cmp(&a.total_seconds));
        let take_n = std::cmp::min(sorted.len(), 8);
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

    // ====== 6. 个性化建议（占位符） ======
    out.push_str("## 效率分析与建议\n\n");
    out.push_str("{{__SUGGESTIONS__}}\n\n");

    // 末尾说明（模型不要删）
    out.push_str("---\n*报告表格、排名、百分比、次数、时长均由程序基于 usage_records 原始记录自动统计生成。*\n");

    out
}

fn build_prompt(report_type: &str, context: &str) -> String {
    let _ = report_type; // 预计算骨架后，所有报告类型走同一套"填空题"格式
    format!(
r#"你的任务是**只写两段中文文字**填入报告骨架，不计算、不改写、不重排任何数字或表格。

【严格操作规则（违反=报告作废）】
1. 输出内容必须严格按照下面的「固定输出格式」，只输出两行 JSON 字段，不要输出任何其它字符、标题、分隔线、表格。
2. 所有数字、百分比、次数、排名、表格、Markdown 标题全部都已经由程序预写死在下方"报告骨架"中，你绝对不能再输出任何与它们相关的文字改写或计算。
3. 不允许在建议中编造"平均单次运行时长""典型运行时长"等任何预计算摘要里没有列出的统计量。
4. 禁止输出任何与"进程运行记录""异常进程""重点关注""访问网页 URL 清单"等预计算骨架中不存在的 section。

【任务内容】
- A. overview（总览描述）：2-4 句中文，总结本周/今日时间花在哪里、最主要的1-2个分类、最主要的应用、最突出的特征（效率高峰/最长专注）。只参考上方【模型写作参考摘要】中已给出的结论性描述，不要自行分析。建议 80-150 字。
- B. suggestions（效率分析与建议）：2-4 条带序号的中文建议（每条 1-2 句）。可围绕：
  1) 专注分析：最长专注时段表现如何
  2) 分心分析：如果识别出游戏/社交/视频等娱乐类应用，建议放在什么时段；如果没有可不提
  3) 效率高峰：把重要工作安排到效率高峰时段
  4) 工作强度/久坐提醒
  5) 结合每日分布，建议把连续过长时段拆分休息
  不要空话套话，每条要有具体动作。

【参考数据】
{context}

【固定输出格式（JSON，必须完全一致，只填字符串值）】
{{{{
  "overview": "【你写的A段，中文】",
  "suggestions": "【你写的B段，用1. 2. 3. 这样的编号格式】"
}}}}
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
///   2) 直接把文字当 overview，suggestions 用默认；
///   3) 全失败返回空字符串兜底。
fn parse_model_json_output(resp: &str) -> (String, String) {
    #[derive(Deserialize)]
    struct Partials {
        overview: Option<String>,
        suggestions: Option<String>,
    }

    let trimmed = resp.trim();
    // 尝试 1：直接解析整个返回
    if let Ok(v) = serde_json::from_str::<Partials>(trimmed) {
        return (
            v.overview.unwrap_or_default().trim().to_string(),
            v.suggestions.unwrap_or_default().trim().to_string(),
        );
    }
    // 尝试 2：截取第一个 { ... } 块
    if let (Some(open), Some(close)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if open < close {
            if let Ok(v) = serde_json::from_str::<Partials>(&trimmed[open..=close]) {
                return (
                    v.overview.unwrap_or_default().trim().to_string(),
                    v.suggestions.unwrap_or_default().trim().to_string(),
                );
            }
        }
    }
    // 兜底：把返回作为 overview，suggestions 给默认
    let fallback_overview = if trimmed.is_empty() {
        "（本次生成未返回总览描述）".to_string()
    } else {
        trimmed.lines().take(6).collect::<Vec<_>>().join(" ").trim().to_string()
    };
    let fallback_suggestions = "1. 将重要工作安排在效率高峰时段，连续专注超过 90 分钟后主动安排 10 分钟休息。\n\
2. 若使用过程中出现游戏、社交、视频等分心源，可集中安排在每日固定时段以减少对专注的打断。\n\
3. 根据每日时长分布，把同类任务安排在同一天的连续时段，减少任务切换带来的认知成本。".to_string();
    (fallback_overview, fallback_suggestions)
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
        }
    };

    let context = precompute_report_blocks(&data);
    let prompt = build_prompt(&params.report_type, &context);

    // ——— Step 1：构建长时间请求客户端，用 IPv4 直连 + 5 分钟总超时 ———
    let client = build_ollama_client(GENERATE_TIMEOUT)?;

    // 降低 temperature：模型只需要做简单的中文措辞润色，不需要发散
    // 减少 num_predict：只需要 2 段中文短文（约 300-500 字），512 token 绰绰有余，降低成本和出错概率
    let body = serde_json::json!({
        "model": model_name,
        "prompt": prompt,
        "stream": false,
        "options": {
            "temperature": 0.35,
            "num_predict": 600
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

    // ——— Step 4：解析模型返回的 JSON → 取 overview / suggestions → 与程序预渲染骨架合并 ———
    let (overview, suggestions) = parse_model_json_output(&ollama_resp.response);
    // 最终合并：把两处占位符替换为模型的文字
    let content_md = context
        .replacen("{{__OVERVIEW__}}", &overview, 1)
        .replacen("{{__SUGGESTIONS__}}", &suggestions, 1);

    let (periodicity, title) = periodicity_and_title(&params.report_type, &params.start_date, &params.end_date);

    let report_id = {
        let db_guard = db.lock().map_err(|e| format!("DB lock error: {}", e))?;
        db_guard
            .insert_or_update_report(&params.report_type, periodicity, &params.start_date, &params.end_date, &title, &content_md)
            .map_err(|e| format!("保存报告失败: {}", e))?
    };

    Ok((report_id, content_md))
}
