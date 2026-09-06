use chrono::{Datelike, Local, NaiveDate, Timelike, Weekday};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

use crate::database::Database;
use crate::ollama::{self, GenerateReportParams};

/// 30 秒粒度的 tick；对于小时级的触发完全足够且 CPU 占用可忽略
const TICK_SECS: u64 = 30;

fn this_week_monday(today: NaiveDate) -> NaiveDate {
    // chrono Weekday: Mon=0 .. Sun=6；num_days_from_monday 正好
    let offset_days: i64 = today.weekday().num_days_from_monday() as i64;
    today - chrono::Duration::days(offset_days)
}

fn today_ymd(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

pub async fn start_scheduler(db: Arc<Mutex<Database>>) {
    tokio::spawn(async move {
        // 已执行的任务指纹（避免同一时间窗内重复触发）
        let mut fired: HashSet<String> = HashSet::new();

        loop {
            let now = Local::now();
            let hour = now.hour();
            let minute = now.minute();
            let today = now.date_naive();
            let yesterday = today - chrono::Duration::days(1);
            let weekday = today.weekday();

            // ----------------------------------------------------------
            // T1: 每天 00:00 ~ 00:02 生成"昨天"的 daily summary
            // ----------------------------------------------------------
            if hour == 0 && minute <= 2 {
                let key = format!("SUM:{}", today_ymd(yesterday));
                if !fired.contains(&key) {
                    fired.insert(key.clone());
                    let db_clone = Arc::clone(&db);
                    let yesterday_date = yesterday;
                    let yesterday_str = today_ymd(yesterday);
                    eprintln!("[Scheduler][T1] 触发每日摘要: {}", yesterday_str);
                    let handle = tokio::task::spawn_blocking(move || {
                        let db = db_clone.lock().unwrap();
                        match db.generate_daily_summary(&yesterday_date) {
                            Ok(_) => eprintln!("[Scheduler][T1] 每日摘要 OK: {}", yesterday_str),
                            Err(e) => eprintln!("[Scheduler][T1] 每日摘要失败 ({}): {}", yesterday_str, e),
                        }
                    });
                    if let Err(e) = handle.await {
                        eprintln!("[Scheduler][T1] 任务 panic: {}", e);
                    }
                }
            }

            // ----------------------------------------------------------
            // T2: 每天 18:30 ~ 18:32 生成标准日报（今天 00:00~当前）
            // ----------------------------------------------------------
            if hour == 18 && (minute == 30 || minute == 31) {
                let key = format!("DRL:{}", today_ymd(today));
                if !fired.contains(&key) {
                    fired.insert(key.clone());
                    let db_clone = Arc::clone(&db);
                    let today_str = today_ymd(today);
                    eprintln!("[Scheduler][T2] 触发标准日报: {}", today_str);
                    tokio::spawn(async move {
                        let params = GenerateReportParams {
                            report_type: "standard".to_string(),
                            start_date: today_str.clone(),
                            end_date: today_str,
                            model: None,
                        };
                        match ollama::generate_and_save_report(&db_clone, params).await {
                            Ok((id, _)) => eprintln!("[Scheduler][T2] 标准日报已入库, id={}", id),
                            Err(e) => eprintln!("[Scheduler][T2] 标准日报生成失败: {}", e),
                        }
                    });
                }
            }

            // ----------------------------------------------------------
            // T3: 每周日 15:00 ~ 15:02 生成标准周报（本周一 ~ 本周日）
            // ----------------------------------------------------------
            if weekday == Weekday::Sun && hour == 15 && (minute == 0 || minute == 1) {
                let week_start = this_week_monday(today);
                let week_end = today; // 周日
                let key = format!("WRL:{}", today_ymd(week_start));
                if !fired.contains(&key) {
                    fired.insert(key.clone());
                    let db_clone = Arc::clone(&db);
                    let start_str = today_ymd(week_start);
                    let end_str = today_ymd(week_end);
                    eprintln!("[Scheduler][T3] 触发标准周报: {} ~ {}", start_str, end_str);
                    tokio::spawn(async move {
                        let params = GenerateReportParams {
                            report_type: "standard".to_string(),
                            start_date: start_str.clone(),
                            end_date: end_str.clone(),
                            model: None,
                        };
                        match ollama::generate_and_save_report(&db_clone, params).await {
                            Ok((id, _)) => eprintln!(
                                "[Scheduler][T3] 标准周报已入库, id={}, {}~{}",
                                id, start_str, end_str
                            ),
                            Err(e) => eprintln!("[Scheduler][T3] 标准周报生成失败: {}", e),
                        }
                    });
                }
            }

            // 防 fired 集合无限增长：每天只可能留 3 个 key，每周日多一个，基本不会爆；
            // 但仍做一次简单清理：大小超过 200 就直接清，反正下次命中时间已过
            if fired.len() > 200 {
                fired.clear();
            }

            sleep(Duration::from_secs(TICK_SECS)).await;
        }
    });
}
