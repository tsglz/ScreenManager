use chrono::{Datelike, Local, NaiveDate, NaiveDateTime};
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: i64,
    pub process_name: String,
    pub window_title: String,
    pub start_time: String,
    pub end_time: String,
    pub duration_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySummary {
    pub id: i64,
    pub date: String,
    pub process_name: String,
    pub duration_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateStats {
    pub date: String,
    pub total_duration: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeekStats {
    pub week_start: String,
    pub week_end: String,
    pub total_duration: i64,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self> {
        let app_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ScreenManager");

        std::fs::create_dir_all(&app_dir).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Null,
                Box::new(e),
            )
        })?;

        let db_path = app_dir.join("usage.db");
        let conn = Connection::open(&db_path)?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        Self::create_tables(&conn)?;

        Ok(Self { conn })
    }

    fn create_tables(conn: &Connection) -> Result<()> {
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS usage_records (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                process_name TEXT NOT NULL,
                window_title TEXT NOT NULL,
                start_time TEXT NOT NULL,
                end_time TEXT NOT NULL,
                duration_seconds INTEGER NOT NULL
            )
            "#,
            [],
        )?;

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS daily_summary (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                process_name TEXT NOT NULL,
                duration_seconds INTEGER NOT NULL,
                UNIQUE(date, process_name)
            )
            "#,
            [],
        )?;

        conn.execute(
            r#"
            CREATE INDEX IF NOT EXISTS idx_usage_records_start_time
            ON usage_records(start_time)
            "#,
            [],
        )?;

        conn.execute(
            r#"
            CREATE INDEX IF NOT EXISTS idx_daily_summary_date
            ON daily_summary(date)
            "#,
            [],
        )?;

        Ok(())
    }

    fn format_datetime(dt: &NaiveDateTime) -> String {
        dt.format("%Y-%m-%dT%H:%M:%S").to_string()
    }

    pub fn insert_record(&self, record: &UsageRecord) -> Result<i64> {
        self.conn.execute(
            r#"
            INSERT INTO usage_records (
                process_name,
                window_title,
                start_time,
                end_time,
                duration_seconds
            ) VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![
                record.process_name,
                record.window_title,
                record.start_time.clone(),
                record.end_time.clone(),
                record.duration_seconds
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn generate_daily_summary(&self, date: &NaiveDate) -> Result<()> {
        let start_of_day = date.and_hms_opt(0, 0, 0).unwrap();
        let end_of_day = date.and_hms_opt(23, 59, 59).unwrap();

        let start_str = Self::format_datetime(&start_of_day);
        let end_str = Self::format_datetime(&end_of_day);
        let date_str = date.format("%Y-%m-%d").to_string();

        self.conn.execute(
            "DELETE FROM daily_summary WHERE date = ?1",
            params![date_str],
        )?;

        self.conn.execute(
            r#"
            INSERT INTO daily_summary (date, process_name, duration_seconds)
            SELECT ?1, process_name, SUM(duration_seconds) as total_duration
            FROM usage_records
            WHERE start_time >= ?2 AND start_time <= ?3
            GROUP BY process_name
            "#,
            params![date_str, start_str, end_str],
        )?;

        Ok(())
    }

    pub fn get_today_total_duration(&self) -> Result<i64> {
        let today = Local::now().date_naive();
        let start_of_day = today.and_hms_opt(0, 0, 0).unwrap();
        let end_of_day = today.and_hms_opt(23, 59, 59).unwrap();

        let start_str = Self::format_datetime(&start_of_day);
        let end_str = Self::format_datetime(&end_of_day);

        let mut stmt = self.conn.prepare(
            r#"
            SELECT COALESCE(SUM(duration_seconds), 0)
            FROM usage_records
            WHERE start_time >= ?1 AND start_time <= ?2
            "#,
        )?;

        let result: i64 = stmt.query_row(params![start_str, end_str], |row| row.get(0))?;
        Ok(result)
    }

    pub fn get_top_apps_today(&self, limit: usize) -> Result<Vec<(String, i64)>> {
        let today = Local::now().date_naive();
        let start_of_day = today.and_hms_opt(0, 0, 0).unwrap();
        let end_of_day = today.and_hms_opt(23, 59, 59).unwrap();

        let start_str = Self::format_datetime(&start_of_day);
        let end_str = Self::format_datetime(&end_of_day);

        let mut stmt = self.conn.prepare(
            r#"
            SELECT process_name, SUM(duration_seconds) as total_duration
            FROM usage_records
            WHERE start_time >= ?1 AND start_time <= ?2
            GROUP BY process_name
            ORDER BY total_duration DESC
            LIMIT ?3
            "#,
        )?;

        let results = stmt.query_map(params![start_str, end_str, limit], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;

        results.collect()
    }

    pub fn get_hourly_distribution_today(&self) -> Result<Vec<(i32, i64)>> {
        let today = Local::now().date_naive();

        let mut result = Vec::with_capacity(24);

        for hour in 0..24 {
            let start_time = today.and_hms_opt(hour, 0, 0).unwrap();
            let end_time = today.and_hms_opt(hour, 59, 59).unwrap();

            let start_str = Self::format_datetime(&start_time);
            let end_str = Self::format_datetime(&end_time);

            let mut stmt = self.conn.prepare(
                r#"
                SELECT COALESCE(SUM(duration_seconds), 0)
                FROM usage_records
                WHERE start_time >= ?1 AND start_time <= ?2
                "#,
            )?;

            let duration: i64 = stmt.query_row(params![start_str, end_str], |row| row.get(0))?;
            result.push((hour as i32, duration));
        }

        Ok(result)
    }

    pub fn get_recent_records(&self, limit: usize) -> Result<Vec<UsageRecord>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, process_name, window_title, start_time, end_time, duration_seconds
            FROM usage_records
            ORDER BY start_time DESC
            LIMIT ?1
            "#,
        )?;

        let records = stmt.query_map(params![limit], |row| {
            Ok(UsageRecord {
                id: row.get(0)?,
                process_name: row.get(1)?,
                window_title: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                duration_seconds: row.get(5)?,
            })
        })?;

        records.collect()
    }

    pub fn get_daily_total(&self, date: &str) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            "SELECT COALESCE(SUM(duration_seconds), 0) FROM daily_summary WHERE date = ?1",
        )?;
        let result: i64 = stmt.query_row(params![date], |row| row.get(0))?;
        Ok(result)
    }

    pub fn get_daily_top_apps(&self, date: &str, limit: usize) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT process_name, duration_seconds
            FROM daily_summary
            WHERE date = ?1
            ORDER BY duration_seconds DESC
            LIMIT ?2
            "#,
        )?;
        let results = stmt.query_map(params![date, limit], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
        results.collect()
    }

    pub fn get_daily_all_apps(&self, date: &str) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT process_name, duration_seconds
            FROM daily_summary
            WHERE date = ?1
            ORDER BY duration_seconds DESC
            "#,
        )?;
        let results = stmt.query_map(params![date], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
        results.collect()
    }

    pub fn get_current_week_stats(&self) -> Result<WeekStats> {
        let today = Local::now().date_naive();
        let week_start = today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64);
        let week_end = week_start + chrono::Duration::days(6);

        let week_start_str = week_start.format("%Y-%m-%d").to_string();
        let week_end_str = week_end.format("%Y-%m-%d").to_string();

        let mut stmt = self.conn.prepare(
            r#"
            SELECT COALESCE(SUM(duration_seconds), 0)
            FROM daily_summary
            WHERE date >= ?1 AND date <= ?2
            "#,
        )?;
        let total: i64 = stmt.query_row(params![week_start_str, week_end_str], |row| row.get(0))?;

        Ok(WeekStats {
            week_start: week_start_str,
            week_end: week_end_str,
            total_duration: total,
        })
    }

    pub fn get_week_top_apps(&self, week_start: &str, week_end: &str, limit: usize) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT process_name, SUM(duration_seconds) as total
            FROM daily_summary
            WHERE date >= ?1 AND date <= ?2
            GROUP BY process_name
            ORDER BY total DESC
            LIMIT ?3
            "#,
        )?;
        let results = stmt.query_map(params![week_start, week_end, limit], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
        results.collect()
    }

    pub fn get_week_daily_stats(&self, week_start: &str, week_end: &str) -> Result<Vec<DateStats>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT date, SUM(duration_seconds) as total
            FROM daily_summary
            WHERE date >= ?1 AND date <= ?2
            GROUP BY date
            ORDER BY date ASC
            "#,
        )?;
        let results = stmt.query_map(params![week_start, week_end], |row| {
            Ok(DateStats {
                date: row.get(0)?,
                total_duration: row.get(1)?,
            })
        })?;
        results.collect()
    }

    pub fn get_previous_week_stats(&self) -> Result<WeekStats> {
        let today = Local::now().date_naive();
        let current_week_start = today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64);
        let prev_week_end = current_week_start - chrono::Duration::days(1);
        let prev_week_start = prev_week_end - chrono::Duration::days(6);

        let week_start_str = prev_week_start.format("%Y-%m-%d").to_string();
        let week_end_str = prev_week_end.format("%Y-%m-%d").to_string();

        let mut stmt = self.conn.prepare(
            r#"
            SELECT COALESCE(SUM(duration_seconds), 0)
            FROM daily_summary
            WHERE date >= ?1 AND date <= ?2
            "#,
        )?;
        let total: i64 = stmt.query_row(params![week_start_str, week_end_str], |row| row.get(0))?;

        Ok(WeekStats {
            week_start: week_start_str,
            week_end: week_end_str,
            total_duration: total,
        })
    }

    pub fn get_current_month_stats(&self) -> Result<DateStats> {
        let today = Local::now().date_naive();
        let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
        let month_end = if today.month() == 12 {
            NaiveDate::from_ymd_opt(today.year() + 1, 1, 1).unwrap() - chrono::Duration::days(1)
        } else {
            NaiveDate::from_ymd_opt(today.year(), today.month() + 1, 1).unwrap() - chrono::Duration::days(1)
        };

        let month_start_str = month_start.format("%Y-%m-%d").to_string();
        let month_end_str = month_end.format("%Y-%m-%d").to_string();

        let mut stmt = self.conn.prepare(
            r#"
            SELECT COALESCE(SUM(duration_seconds), 0)
            FROM daily_summary
            WHERE date >= ?1 AND date <= ?2
            "#,
        )?;
        let total: i64 = stmt.query_row(params![month_start_str, month_end_str], |row| row.get(0))?;

        Ok(DateStats {
            date: month_start_str,
            total_duration: total,
        })
    }

    pub fn get_month_top_apps(&self, month_start: &str, month_end: &str, limit: usize) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT process_name, SUM(duration_seconds) as total
            FROM daily_summary
            WHERE date >= ?1 AND date <= ?2
            GROUP BY process_name
            ORDER BY total DESC
            LIMIT ?3
            "#,
        )?;
        let results = stmt.query_map(params![month_start, month_end, limit], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
        results.collect()
    }

    pub fn get_month_daily_stats(&self, month_start: &str, month_end: &str) -> Result<Vec<DateStats>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT date, SUM(duration_seconds) as total
            FROM daily_summary
            WHERE date >= ?1 AND date <= ?2
            GROUP BY date
            ORDER BY date ASC
            "#,
        )?;
        let results = stmt.query_map(params![month_start, month_end], |row| {
            Ok(DateStats {
                date: row.get(0)?,
                total_duration: row.get(1)?,
            })
        })?;
        results.collect()
    }

    pub fn get_previous_month_stats(&self) -> Result<DateStats> {
        let today = Local::now().date_naive();
        let prev_month = if today.month() == 1 {
            NaiveDate::from_ymd_opt(today.year() - 1, 12, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(today.year(), today.month() - 1, 1).unwrap()
        };
        let prev_month_end = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap() - chrono::Duration::days(1);

        let month_start_str = prev_month.format("%Y-%m-%d").to_string();
        let month_end_str = prev_month_end.format("%Y-%m-%d").to_string();

        let mut stmt = self.conn.prepare(
            r#"
            SELECT COALESCE(SUM(duration_seconds), 0)
            FROM daily_summary
            WHERE date >= ?1 AND date <= ?2
            "#,
        )?;
        let total: i64 = stmt.query_row(params![month_start_str, month_end_str], |row| row.get(0))?;

        Ok(DateStats {
            date: month_start_str,
            total_duration: total,
        })
    }

    pub fn get_date_range_stats(&self, start_date: &str, end_date: &str) -> Result<Vec<DateStats>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT date, SUM(duration_seconds) as total
            FROM daily_summary
            WHERE date >= ?1 AND date <= ?2
            GROUP BY date
            ORDER BY date ASC
            "#,
        )?;
        let results = stmt.query_map(params![start_date, end_date], |row| {
            Ok(DateStats {
                date: row.get(0)?,
                total_duration: row.get(1)?,
            })
        })?;
        results.collect()
    }
}

mod dirs {
    use std::path::PathBuf;

    pub fn data_local_dir() -> Option<PathBuf> {
        if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
            Some(PathBuf::from(appdata))
        } else {
            None
        }
    }
}