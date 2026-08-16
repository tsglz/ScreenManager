use chrono::NaiveDate;
use crate::database::{CategoryStats, Database, UsageRecord, DateStats};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

const OLLAMA_URL: &str = "http://localhost:11434/api/generate";
const MODEL_NAME: &str = "qwen3:4b-fp16";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportData {
    pub start_date: String,
    pub end_date: String,
    pub total_duration: i64,
    pub top_apps: Vec<(String, i64)>,
    pub category_stats: Vec<CategoryStats>,
    pub records: Vec<UsageRecord>,
    pub daily_stats: Vec<DateStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateReportParams {
    pub report_type: String,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
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

fn build_context(data: &ReportData) -> String {
    let mut ctx = String::new();

    ctx.push_str(&format!("时间范围：{} 至 {}\n", data.start_date, data.end_date));
    ctx.push_str(&format!("总工作时长：{}\n\n", format_duration_hm(data.total_duration)));

    ctx.push_str("每日工作时长：\n");
    for day in &data.daily_stats {
        ctx.push_str(&format!("  {}：{}\n", day.date, format_duration_hm(day.total_duration)));
    }
    ctx.push('\n');

    ctx.push_str("应用使用排行（Top 20）：\n");
    for (i, (name, dur)) in data.top_apps.iter().enumerate() {
        ctx.push_str(&format!("  {}. {} — {}\n", i + 1, name, format_duration_hm(*dur)));
    }
    ctx.push('\n');

    ctx.push_str("分类统计：\n");
    let cat_total: i64 = data.category_stats.iter().map(|c| c.duration_seconds).sum();
    for cat in &data.category_stats {
        let pct = if cat_total > 0 {
            (cat.duration_seconds as f64 / cat_total as f64) * 100.0
        } else {
            0.0
        };
        ctx.push_str(&format!(
            "  {}：{} ({:.1}%)\n",
            cat.category_name,
            format_duration_hm(cat.duration_seconds),
            pct
        ));
    }
    ctx.push('\n');

    ctx.push_str("详细活动记录（按时长排序，Top 50）：\n");
    for (i, record) in data.records.iter().enumerate() {
        ctx.push_str(&format!(
            "  {}. [{}] {} — {} ({})\n",
            i + 1,
            format_time_brief(&record.start_time),
            record.process_name,
            record.window_title,
            format_duration_hm(record.duration_seconds)
        ));
    }

    ctx
}

fn build_prompt(report_type: &str, context: &str) -> String {
    let role = "你是一位专业的工作报告助手，根据用户的电脑使用记录数据生成结构化的工作报告。";

    let (type_name, instructions) = match report_type {
        "pomodoro" => (
            "番茄钟聚类报告",
            "请将工作活动按类型进行聚类分析，不要按时间线排列。\
将活动归纳为3-6个工作集群（如：开发工作、沟通协作、文档撰写等），\
每个集群列出包含的应用和总时长，并分析该集群的工作重点。\
最后给出今日工作效率评估和建议。"
        ),
        "concise" => (
            "简洁日报",
            "只列出今日最关键的3-5项工作内容，适合快速汇报。\
每项工作用一句话概括，包含工作内容和成果。\
不要罗列所有应用，只突出重点。\
格式简洁明了，便于上级快速了解工作进展。"
        ),
        "tech" => (
            "技术日报",
            "侧重于代码开发和技术问题。\
重点描述：使用的开发工具和IDE、技术调试和问题解决、代码编写相关的工作。\
识别可能的技术挑战和解决方案。\
适合技术团队内部的日报交流。"
        ),
        "project" => (
            "项目日报",
            "按项目维度组织工作内容。\
根据窗口标题和应用推断涉及的 different 项目，\
每个项目列出今日投入时长、主要工作内容和进展。\
如果无法明确区分项目，按工作主题进行归纳。"
        ),
        "standard" => (
            "标准日报",
            "按工作类别归纳今日已完成的工作。\
类别包括但不限于：开发、会议沟通、文档撰写、学习研究、系统维护等。\
每个类别下列出具体工作内容和耗时。\
最后附上明日工作计划建议（基于今日工作推断）。"
        ),
        _ => ("工作报告", "请根据数据生成一份结构清晰的工作报告。"),
    };

    format!(
        "{role}\n\n\
请根据以下电脑使用记录数据，生成一份【{type_name}】。\n\n\
报告要求：\n{instructions}\n\n\
使用数据：\n{context}\n\n\
请用中文输出报告，使用 Markdown 格式。不要输出思考过程，直接输出报告内容。"
    )
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

pub async fn generate_and_save_report(
    db: &Arc<Mutex<Database>>,
    params: GenerateReportParams,
) -> Result<(i64, String), String> {
    let data = {
        let db_guard = db.lock().map_err(|e| format!("DB lock error: {}", e))?;
        let total = db_guard.get_range_total(&params.start_date, &params.end_date)
            .map_err(|e| format!("Query total error: {}", e))?;
        let top_apps = db_guard.get_range_top_apps(&params.start_date, &params.end_date, 20)
            .map_err(|e| format!("Query top apps error: {}", e))?;
        let category_stats = db_guard.get_range_category_stats(&params.start_date, &params.end_date)
            .map_err(|e| format!("Query category stats error: {}", e))?;
        let records = db_guard.get_records_for_date_range(&params.start_date, &params.end_date, 50)
            .map_err(|e| format!("Query records error: {}", e))?;
        let daily_stats = db_guard.get_date_range_stats(&params.start_date, &params.end_date)
            .map_err(|e| format!("Query daily stats error: {}", e))?;

        if total == 0 {
            return Err("所选时间范围内没有工作数据".to_string());
        }

        ReportData {
            start_date: params.start_date.clone(),
            end_date: params.end_date.clone(),
            total_duration: total,
            top_apps,
            category_stats,
            records,
            daily_stats,
        }
    };

    let context = build_context(&data);
    let prompt = build_prompt(&params.report_type, &context);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let body = serde_json::json!({
        "model": MODEL_NAME,
        "prompt": prompt,
        "stream": false,
        "options": {
            "temperature": 0.7,
            "num_predict": 2048
        }
    });

    let response = client
        .post(OLLAMA_URL)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("无法连接到 Ollama（{}）：{}\n请确认 Ollama 已启动并运行 {} 模型", OLLAMA_URL, e, MODEL_NAME))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Ollama 返回错误 ({}): {}", status, text));
    }

    let ollama_resp: OllamaResponse = response
        .json()
        .await
        .map_err(|e| format!("解析 Ollama 响应失败: {}", e))?;

    let content_md = ollama_resp.response;

    let (periodicity, title) = periodicity_and_title(&params.report_type, &params.start_date, &params.end_date);

    let report_id = {
        let db_guard = db.lock().map_err(|e| format!("DB lock error: {}", e))?;
        db_guard
            .insert_or_update_report(&params.report_type, periodicity, &params.start_date, &params.end_date, &title, &content_md)
            .map_err(|e| format!("保存报告失败: {}", e))?
    };

    Ok((report_id, content_md))
}
