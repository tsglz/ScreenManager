use chrono::Local;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

use crate::database::Database;

pub async fn start_scheduler(db: Arc<Mutex<Database>>) {
    tokio::spawn(async move {
        loop {
            let now = Local::now();
            let midnight = now.date_naive() + chrono::Duration::days(1);
            let midnight_dt = midnight.and_hms_opt(0, 0, 0).unwrap();
            let duration_until_midnight = (midnight_dt - now.naive_local()).num_seconds();

            eprintln!("[Scheduler] Next midnight in {} seconds", duration_until_midnight);

            sleep(Duration::from_secs(duration_until_midnight as u64)).await;

            let yesterday = Local::now().date_naive() - chrono::Duration::days(1);
            eprintln!("[Scheduler] Generating daily summary for {}", yesterday);

            let db_clone = Arc::clone(&db);
            let yesterday_str = yesterday.format("%Y-%m-%d").to_string();
            tokio::task::spawn_blocking(move || {
                let db = db_clone.lock().unwrap();
                match db.generate_daily_summary(&yesterday) {
                    Ok(_) => eprintln!("[Scheduler] Daily summary generated successfully"),
                    Err(e) => eprintln!("[Scheduler] Failed to generate daily summary: {}", e),
                }
            }).await.unwrap();
        }
    });
}

pub fn calculate_midnight_duration() -> Duration {
    let now = Local::now();
    let midnight = now.date_naive() + chrono::Duration::days(1);
    let midnight_dt = midnight.and_hms_opt(0, 0, 0).unwrap();
    let seconds = (midnight_dt - now.naive_local()).num_seconds();
    Duration::from_secs(seconds as u64)
}