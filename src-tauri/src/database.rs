use chrono::{Datelike, Duration as ChronoDur, Local, NaiveDate, NaiveDateTime};
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const CURRENT_SCHEMA_VERSION: i32 = 5;

/// 默认分类列表版本号。每当 `init_default_categories` 中的默认映射发生变化时递增。
/// 升级时会用 INSERT OR REPLACE 刷新已有默认条目，修复历史遗留的错配分类。
pub const DEFAULT_CATEGORIES_VERSION: i32 = 2;

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
pub struct _DailySummary {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryStats {
    pub category_name: String,
    pub duration_seconds: i64,
}

/// 单应用在所选时间范围内的聚合统计（含出现次数，避免AI自行估算）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppAggStat {
    pub process_name: String,
    pub total_seconds: i64,
    /// usage_records 中该应用出现的记录条数（一条记录即"启动一次会话片段"）
    pub record_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyHeatmapEntry {
    pub date: String,
    pub hour: i32,
    pub duration_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: i64,
    pub report_type: String,
    pub periodicity: String,
    pub start_date: String,
    pub end_date: String,
    pub title: String,
    pub content_md: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportListItem {
    pub id: i64,
    pub report_type: String,
    pub periodicity: String,
    pub start_date: String,
    pub end_date: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportListResult {
    pub items: Vec<ReportListItem>,
    pub total: i64,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self> {
        let app_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ScreenTime");

        std::fs::create_dir_all(&app_dir).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Null, Box::new(e))
        })?;

        let db_path = app_dir.join("screen_time.db");
        let conn = Connection::open(&db_path)?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        let schema_version = Self::get_schema_version(&conn)?;

        Self::run_migrations(&conn, schema_version)?;

        Ok(Self { conn })
    }

    fn get_schema_version(conn: &Connection) -> Result<i32> {
        let has_schema_version = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='schema_version'",
                [],
                |_| Ok(()),
            )
            .is_ok();

        if has_schema_version {
            let version: i32 = conn.query_row(
                "SELECT version FROM schema_version ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )?;
            Ok(version)
        } else {
            Ok(0)
        }
    }

    fn run_migrations(conn: &Connection, from_version: i32) -> Result<()> {
        if from_version < CURRENT_SCHEMA_VERSION {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS schema_version (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    version INTEGER NOT NULL,
                    applied_at TEXT NOT NULL
                )",
                [],
            )?;
        }

        if from_version < 1 {
            Self::migration_v1(conn)?;
            conn.execute(
                "INSERT INTO schema_version (version, applied_at) VALUES (1, datetime('now'))",
                [],
            )?;
        }

        if from_version < 2 {
            Self::migration_v2(conn)?;
            conn.execute(
                "INSERT INTO schema_version (version, applied_at) VALUES (2, datetime('now'))",
                [],
            )?;
        }

        if from_version < 3 {
            Self::migration_v3(conn)?;
            conn.execute(
                "INSERT INTO schema_version (version, applied_at) VALUES (3, datetime('now'))",
                [],
            )?;
        }

        if from_version < 4 {
            Self::migration_v4(conn)?;
            conn.execute(
                "INSERT INTO schema_version (version, applied_at) VALUES (4, datetime('now'))",
                [],
            )?;
        }

        if from_version < 5 {
            println!("Running migration v5...");
            Self::migration_v5(conn)?;
            conn.execute(
                "INSERT INTO schema_version (version, applied_at) VALUES (5, datetime('now'))",
                [],
            )?;
        }

        Ok(())
    }

    fn migration_v1(conn: &Connection) -> Result<()> {
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
            CREATE TABLE IF NOT EXISTS app_categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                process_name TEXT NOT NULL UNIQUE,
                category_name TEXT NOT NULL
            )
            "#,
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_usage_records_start_time ON usage_records(start_time)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_daily_summary_date ON daily_summary(date)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_app_categories_process_name ON app_categories(process_name)",
            [],
        )?;

        Ok(())
    }

    fn migration_v2(conn: &Connection) -> Result<()> {
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                color TEXT,
                icon TEXT
            )
            "#,
            [],
        )?;

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )
            "#,
            [],
        )?;

        Ok(())
    }

    fn migration_v3(conn: &Connection) -> Result<()> {
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS update_info (
                id INTEGER PRIMARY KEY,
                skip_version TEXT,
                last_check TEXT
            )
            "#,
            [],
        )?;

        conn.execute(
            "INSERT OR IGNORE INTO update_info (id, skip_version, last_check) VALUES (1, NULL, NULL)",
            [],
        )?;

        Ok(())
    }

    fn migration_v4(conn: &Connection) -> Result<()> {
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS record_project_overrides (
                record_id  INTEGER PRIMARY KEY,
                project    TEXT NOT NULL,
                source     TEXT NOT NULL DEFAULT 'user',
                updated_at TEXT NOT NULL
            )
            "#,
            [],
        )?;

        conn.execute(
            "INSERT OR IGNORE INTO config (key, value) VALUES ('session_idle_threshold', '900')",
            [],
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO config (key, value) VALUES ('session_switch_threshold', '300')",
            [],
        )?;

        Ok(())
    }

    fn migration_v5(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS reports (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                report_type TEXT    NOT NULL,
                periodicity TEXT    NOT NULL,
                start_date  TEXT    NOT NULL,
                end_date    TEXT    NOT NULL,
                title       TEXT    NOT NULL,
                content_md  TEXT    NOT NULL,
                created_at  TEXT    NOT NULL,
                updated_at  TEXT    NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_reports_created ON reports(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_reports_type    ON reports(report_type, periodicity);
            "#,
        )?;
        Ok(())
    }

    pub fn _get_schema_version_current(&self) -> Result<i32> {
        Self::get_schema_version(&self.conn)
    }

    pub fn repair_abnormal_records(&self) -> Result<()> {
        self.conn.execute(
            r#"
            UPDATE usage_records
            SET end_time = start_time,
                duration_seconds = 0
            WHERE end_time IS NULL OR end_time = '' OR duration_seconds < 0
            "#,
            [],
        )?;
        Ok(())
    }

    pub fn init_default_categories(&self) -> Result<()> {
        let default_categories = vec![
            // 浏览器
            ("chrome.exe", "浏览器"),
            ("msedge.exe", "浏览器"),
            ("firefox.exe", "浏览器"),
            ("opera.exe", "浏览器"),
            ("brave.exe", "浏览器"),
            ("vivaldi.exe", "浏览器"),
            ("360se.exe", "浏览器"),
            ("360chrome.exe", "浏览器"),
            ("QQBrowser.exe", "浏览器"),
            ("SogouExplorer.exe", "浏览器"),
            ("liebao.exe", "浏览器"),
            ("maxthon.exe", "浏览器"),
            // 开发工具
            ("code.exe", "开发工具"),
            ("idea64.exe", "开发工具"),
            ("pycharm64.exe", "开发工具"),
            ("webstorm64.exe", "开发工具"),
            ("Trae CN.exe", "开发工具"),
            ("clion64.exe", "开发工具"),
            ("rustrover.exe", "开发工具"),
            ("goland64.exe", "开发工具"),
            ("rider64.exe", "开发工具"),
            ("phpstorm64.exe", "开发工具"),
            ("datagrip64.exe", "开发工具"),
            ("devenv.exe", "开发工具"),
            ("javaw.exe", "开发工具"),
            ("Cursor.exe", "开发工具"),
            ("python.exe", "开发工具"),
            ("node.exe", "开发工具"),
            ("sublime_text.exe", "开发工具"),
            ("eclipse.exe", "开发工具"),
            ("studio64.exe", "开发工具"),
            ("Docker Desktop.exe", "开发工具"),
            ("postman.exe", "开发工具"),
            ("navicat.exe", "开发工具"),
            ("dbeaver.exe", "开发工具"),
            ("gitkraken.exe", "开发工具"),
            ("sourcetree.exe", "开发工具"),
            ("xshell.exe", "开发工具"),
            ("MobaXterm.exe", "开发工具"),
            // AI 工具
            ("QClaw.exe", "AI工具"),
            ("Cherry Studio.exe", "AI工具"),
            ("claude.exe", "AI工具"),
            ("ollama.exe", "AI工具"),
            ("lmstudio.exe", "AI工具"),
            // 社交通讯
            ("wechat.exe", "社交通讯"),
            ("QQ.exe", "社交通讯"),
            ("tim.exe", "社交通讯"),
            ("dingtalk.exe", "社交通讯"),
            ("Feishu.exe", "社交通讯"),
            ("lark.exe", "社交通讯"),
            ("WeChatAppEx.exe", "社交通讯"),
            ("telegram.exe", "社交通讯"),
            ("slack.exe", "社交通讯"),
            ("discord.exe", "社交通讯"),
            // Steam 游戏
            ("steam.exe", "Steam游戏"),
            ("Cyberpunk2077.exe", "Steam游戏"),
            ("eurotrucks2.exe", "Steam游戏"),
            ("Card Shop Simulator Prologue.exe", "Steam游戏"),
            ("SurvivalLog.exe", "Steam游戏"),
            // 游戏娱乐
            ("EpicGamesLauncher.exe", "游戏娱乐"),
            ("battle.net.exe", "游戏娱乐"),
            ("riotclientservices.exe", "游戏娱乐"),
            ("LeagueClient.exe", "游戏娱乐"),
            ("minecraft.exe", "游戏娱乐"),
            // 音乐
            ("cloudmusic.exe", "音乐"),
            ("spotify.exe", "音乐"),
            ("qqmusic.exe", "音乐"),
            ("kugou.exe", "音乐"),
            ("foobar2000.exe", "音乐"),
            // 视频播放
            ("vlc.exe", "视频播放"),
            ("PotPlayerMini64.exe", "视频播放"),
            ("mpc-hc64.exe", "视频播放"),
            ("mpv.exe", "视频播放"),
            // 知识库
            ("obsidian.exe", "知识库"),
            ("notion.exe", "知识库"),
            ("logseq.exe", "知识库"),
            ("ima.copilot.exe", "知识库"),
            ("onenote.exe", "知识库"),
            // 办公工具
            ("excel.exe", "办公工具"),
            ("winword.exe", "办公工具"),
            ("powerpnt.exe", "办公工具"),
            ("outlook.exe", "办公工具"),
            ("WPSOffice.exe", "办公工具"),
            ("wps.exe", "办公工具"),
            ("et.exe", "办公工具"),
            ("wpp.exe", "办公工具"),
            ("foxitreader.exe", "办公工具"),
            ("Acrobat.exe", "办公工具"),
            ("typora.exe", "办公工具"),
            // 设计工具
            ("photoshop.exe", "设计工具"),
            ("illustrator.exe", "设计工具"),
            ("figma.exe", "设计工具"),
            ("axure.exe", "设计工具"),
            ("inkscape.exe", "设计工具"),
            // 系统工具
            ("Taskmgr.exe", "系统工具"),
            ("Explorer.EXE", "系统工具"),
            ("cmd.exe", "系统工具"),
            ("powershell.exe", "系统工具"),
            ("conhost.exe", "系统工具"),
            ("RuntimeBroker.exe", "系统工具"),
            ("svchost.exe", "系统工具"),
            ("SystemSettings.exe", "系统工具"),
            ("regedit.exe", "系统工具"),
            ("SnippingTool.exe", "系统工具"),
            // ScreenManager
            ("screen-manager.exe", "ScreenManager"),
            ];

        // 版本门控：当默认分类版本升级时，用 INSERT OR REPLACE 刷新旧默认条目，
        // 修复历史遗留的错配（如旧版本写入的 steam→游戏娱乐、QClaw→办公工具等）。
        // 正常运行（版本未变）时仍用 INSERT OR IGNORE，保留用户手动修改的自定义分类。
        // 注意：仅刷新默认列表中的进程；不在默认列表中的自定义条目不受影响。
        let applied_version: i32 = self
            .conn
            .query_row(
                "SELECT value FROM config WHERE key = 'default_categories_version'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let force_update = applied_version < DEFAULT_CATEGORIES_VERSION;

        for (process_name, category_name) in default_categories {
            let sql = if force_update {
                "INSERT OR REPLACE INTO app_categories (process_name, category_name) VALUES (?1, ?2)"
            } else {
                "INSERT OR IGNORE INTO app_categories (process_name, category_name) VALUES (?1, ?2)"
            };
            self.conn.execute(sql, params![process_name, category_name])?;
        }

        if force_update {
            let _ = self.conn.execute(
                "INSERT OR REPLACE INTO config (key, value) VALUES ('default_categories_version', ?1)",
                params![DEFAULT_CATEGORIES_VERSION.to_string()],
            );
        }

        Ok(())
    }

    pub fn get_all_categories(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT process_name, category_name FROM app_categories ORDER BY category_name, process_name",
        )?;
        let results = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        results.collect()
    }

    pub fn get_category_for_app(&self, process_name: &str) -> Result<String> {
        let mut stmt = self
            .conn
            .prepare("SELECT category_name FROM app_categories WHERE process_name = ?1")?;
        match stmt.query_row(params![process_name], |row| row.get::<_, String>(0)) {
            Ok(cat) => Ok(cat),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Ok(Self::auto_classify(process_name).unwrap_or_else(|| "其他".to_string()))
            }
            Err(e) => Err(e),
        }
    }

    pub fn set_app_category(&self, process_name: &str, category_name: &str) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO app_categories (process_name, category_name)
            VALUES (?1, ?2)
            "#,
            params![process_name, category_name],
        )?;
        Ok(())
    }

    fn get_app_category_map(&self) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();
        if let Ok(categories) = self.get_all_categories() {
            for (process_name, category_name) in categories {
                map.insert(process_name.to_lowercase(), category_name);
            }
        }
        map
    }

    /// 根据进程名关键词自动识别分类，用于未在 app_categories 表中登记的应用。
    /// 返回 None 表示无法识别，调用方可回退到“其他”。
    fn auto_classify(process_name: &str) -> Option<String> {
        let lower = process_name.to_lowercase();
        // 关键词按类别分组，首条命中即返回。
        const PATTERNS: &[(&str, &str)] = &[
            // 浏览器
            ("chrome", "浏览器"),
            ("chromium", "浏览器"),
            ("msedge", "浏览器"),
            ("firefox", "浏览器"),
            ("opera", "浏览器"),
            ("brave", "浏览器"),
            ("vivaldi", "浏览器"),
            ("maxthon", "浏览器"),
            ("qqbrowser", "浏览器"),
            ("360se", "浏览器"),
            ("360chrome", "浏览器"),
            ("sogouexplorer", "浏览器"),
            ("liebao", "浏览器"),
            ("ucbrowser", "浏览器"),
            ("waterfox", "浏览器"),
            ("palemoon", "浏览器"),
            // 开发工具
            ("vscode", "开发工具"),
            ("code.exe", "开发工具"),
            ("code - insiders", "开发工具"),
            ("idea", "开发工具"),
            ("pycharm", "开发工具"),
            ("webstorm", "开发工具"),
            ("clion", "开发工具"),
            ("rustrover", "开发工具"),
            ("goland", "开发工具"),
            ("rider", "开发工具"),
            ("phpstorm", "开发工具"),
            ("datagrip", "开发工具"),
            ("devenv", "开发工具"),
            ("visualstudio", "开发工具"),
            ("androidstudio", "开发工具"),
            ("studio64", "开发工具"),
            ("cursor", "开发工具"),
            ("trae", "开发工具"),
            ("fleet", "开发工具"),
            ("neovim", "开发工具"),
            ("gvim", "开发工具"),
            ("vim.exe", "开发工具"),
            ("emacs", "开发工具"),
            ("sublime", "开发工具"),
            ("atom.exe", "开发工具"),
            ("zed.exe", "开发工具"),
            ("eclipse", "开发工具"),
            ("netbeans", "开发工具"),
            ("sourcetree", "开发工具"),
            ("gitkraken", "开发工具"),
            ("fork.exe", "开发工具"),
            ("beyond", "开发工具"),
            ("winscp", "开发工具"),
            ("filezilla", "开发工具"),
            ("putty", "开发工具"),
            ("xshell", "开发工具"),
            ("mobaxterm", "开发工具"),
            ("alacritty", "开发工具"),
            ("wezterm", "开发工具"),
            ("kitty", "开发工具"),
            ("tabby", "开发工具"),
            ("docker", "开发工具"),
            ("postman", "开发工具"),
            ("insomnia", "开发工具"),
            ("dbeaver", "开发工具"),
            ("navicat", "开发工具"),
            ("heidisql", "开发工具"),
            ("redisinsight", "开发工具"),
            ("termius", "开发工具"),
            ("python", "开发工具"),
            ("node.exe", "开发工具"),
            ("javaw", "开发工具"),
            ("cargo", "开发工具"),
            // AI 工具
            ("qclaw", "AI工具"),
            ("cherry studio", "AI工具"),
            ("claude", "AI工具"),
            ("chatgpt", "AI工具"),
            ("ollama", "AI工具"),
            ("lmstudio", "AI工具"),
            // 社交通讯
            ("wechat", "社交通讯"),
            ("weixin", "社交通讯"),
            ("qq.exe", "社交通讯"),
            ("tim.exe", "社交通讯"),
            ("dingtalk", "社交通讯"),
            ("feishu", "社交通讯"),
            ("lark", "社交通讯"),
            ("telegram", "社交通讯"),
            ("whatsapp", "社交通讯"),
            ("slack", "社交通讯"),
            ("discord", "社交通讯"),
            ("skype", "社交通讯"),
            ("msteams", "社交通讯"),
            // Steam 游戏（优先于游戏娱乐，放在前面以命中 steam 相关进程）
            ("steam", "Steam游戏"),
            // 游戏娱乐
            ("epicgames", "游戏娱乐"),
            ("battle.net", "游戏娱乐"),
            ("uplay", "游戏娱乐"),
            ("origin.exe", "游戏娱乐"),
            ("riotclient", "游戏娱乐"),
            ("leagueclient", "游戏娱乐"),
            ("minecraft", "游戏娱乐"),
            ("genshinimpact", "游戏娱乐"),
            // 音乐
            ("cloudmusic", "音乐"),
            ("spotify", "音乐"),
            ("qqmusic", "音乐"),
            ("kugou", "音乐"),
            ("kuwo", "音乐"),
            ("foobar2000", "音乐"),
            ("aimp", "音乐"),
            // 视频播放
            ("vlc", "视频播放"),
            ("potplayer", "视频播放"),
            ("mpc-hc", "视频播放"),
            ("mpc-be", "视频播放"),
            ("mpv.exe", "视频播放"),
            ("smplayer", "视频播放"),
            ("kmplayer", "视频播放"),
            // 知识库
            ("obsidian", "知识库"),
            ("notion", "知识库"),
            ("logseq", "知识库"),
            ("ima.copilot", "知识库"),
            ("onenote", "知识库"),
            ("evernote", "知识库"),
            // 办公工具
            ("excel", "办公工具"),
            ("winword", "办公工具"),
            ("powerpnt", "办公工具"),
            ("outlook", "办公工具"),
            ("wpsoffice", "办公工具"),
            ("wps.exe", "办公工具"),
            ("et.exe", "办公工具"),
            ("wpp.exe", "办公工具"),
            ("foxit", "办公工具"),
            ("acrobat", "办公工具"),
            ("typora", "办公工具"),
            ("notepad++", "办公工具"),
            // 设计工具
            ("photoshop", "设计工具"),
            ("illustrator", "设计工具"),
            ("indesign", "设计工具"),
            ("premiere", "设计工具"),
            ("afterfx", "设计工具"),
            ("figma", "设计工具"),
            ("axure", "设计工具"),
            ("inkscape", "设计工具"),
            ("gimp", "设计工具"),
            ("krita", "设计工具"),
            ("blender", "设计工具"),
            // 系统工具
            ("taskmgr", "系统工具"),
            ("explorer", "系统工具"),
            ("cmd.exe", "系统工具"),
            ("powershell", "系统工具"),
            ("conhost", "系统工具"),
            ("runtimebroker", "系统工具"),
            ("svchost", "系统工具"),
            ("systemsettings", "系统工具"),
            ("regedit", "系统工具"),
            ("mmc.exe", "系统工具"),
            ("snippingtool", "系统工具"),
            ("calc.exe", "系统工具"),
            // ScreenManager
            ("screen-manager", "ScreenManager"),
        ];
        for (kw, cat) in PATTERNS {
            if lower.contains(kw) {
                return Some(cat.to_string());
            }
        }
        None
    }

    /// 合并显式映射与自动识别：优先使用数据库中的显式分类，
    /// 命中失败时调用 auto_classify 进行关键词识别，仍无法识别则返回“其他”。
    fn classify_process(
        process_name: &str,
        category_map: &std::collections::HashMap<String, String>,
    ) -> String {
        if let Some(cat) = category_map.get(&process_name.to_lowercase()) {
            return cat.clone();
        }
        Self::auto_classify(process_name).unwrap_or_else(|| "其他".to_string())
    }

    pub fn get_today_category_stats(&self) -> Result<Vec<CategoryStats>> {
        let today = Local::now().date_naive();
        let start_of_day = today.and_hms_opt(0, 0, 0).unwrap();
        let end_of_day = today.and_hms_opt(23, 59, 59).unwrap();

        let start_str = Self::format_datetime(&start_of_day);
        let end_str = Self::format_datetime(&end_of_day);

        let category_map = self.get_app_category_map();

        let mut stmt = self.conn.prepare(
            r#"
            SELECT process_name, SUM(duration_seconds) as total
            FROM usage_records
            WHERE start_time >= ?1 AND start_time <= ?2
            GROUP BY process_name
            "#,
        )?;

        let mut category_totals: std::collections::HashMap<String, i64> =
            std::collections::HashMap::new();
        let mut results = stmt.query(params![start_str, end_str])?;

        while let Some(row) = results.next()? {
            let process_name: String = row.get(0)?;
            let duration: i64 = row.get(1)?;
            let category = Self::classify_process(&process_name, &category_map);
            *category_totals.entry(category).or_insert(0) += duration;
        }

        let mut stats: Vec<CategoryStats> = category_totals
            .into_iter()
            .map(|(category_name, duration_seconds)| CategoryStats {
                category_name,
                duration_seconds,
            })
            .collect();
        stats.sort_by(|a, b| b.duration_seconds.cmp(&a.duration_seconds));
        Ok(stats)
    }

    pub fn get_daily_category_stats(&self, date: &str) -> Result<Vec<CategoryStats>> {
        let date_obj = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| {
            rusqlite::Error::InvalidParameterName("Invalid date format".to_string())
        })?;
        let start_of_day = date_obj.and_hms_opt(0, 0, 0).unwrap();
        let end_of_day = date_obj.and_hms_opt(23, 59, 59).unwrap();

        let start_str = Self::format_datetime(&start_of_day);
        let end_str = Self::format_datetime(&end_of_day);

        let category_map = self.get_app_category_map();

        let mut stmt = self.conn.prepare(
            r#"
            SELECT process_name, SUM(duration_seconds) as total
            FROM usage_records
            WHERE start_time >= ?1 AND start_time <= ?2
            GROUP BY process_name
            "#,
        )?;

        let mut category_totals: std::collections::HashMap<String, i64> =
            std::collections::HashMap::new();
        let mut results = stmt.query(params![start_str, end_str])?;

        while let Some(row) = results.next()? {
            let process_name: String = row.get(0)?;
            let duration: i64 = row.get(1)?;
            let category = Self::classify_process(&process_name, &category_map);
            *category_totals.entry(category).or_insert(0) += duration;
        }

        let mut stats: Vec<CategoryStats> = category_totals
            .into_iter()
            .map(|(category_name, duration_seconds)| CategoryStats {
                category_name,
                duration_seconds,
            })
            .collect();
        stats.sort_by(|a, b| b.duration_seconds.cmp(&a.duration_seconds));
        Ok(stats)
    }

    pub fn get_week_category_stats(
        &self,
        week_start: &str,
        week_end: &str,
    ) -> Result<Vec<CategoryStats>> {
        let category_map = self.get_app_category_map();

        let mut stmt = self.conn.prepare(
            r#"
            SELECT process_name, SUM(duration_seconds) as total
            FROM daily_summary
            WHERE date >= ?1 AND date <= ?2
            GROUP BY process_name
            "#,
        )?;

        let mut category_totals: std::collections::HashMap<String, i64> =
            std::collections::HashMap::new();
        let mut results = stmt.query(params![week_start, week_end])?;

        while let Some(row) = results.next()? {
            let process_name: String = row.get(0)?;
            let duration: i64 = row.get(1)?;
            let category = Self::classify_process(&process_name, &category_map);
            *category_totals.entry(category).or_insert(0) += duration;
        }

        let mut stats: Vec<CategoryStats> = category_totals
            .into_iter()
            .map(|(category_name, duration_seconds)| CategoryStats {
                category_name,
                duration_seconds,
            })
            .collect();
        stats.sort_by(|a, b| b.duration_seconds.cmp(&a.duration_seconds));
        Ok(stats)
    }

    pub fn get_month_category_stats(
        &self,
        month_start: &str,
        month_end: &str,
    ) -> Result<Vec<CategoryStats>> {
        let category_map = self.get_app_category_map();

        let mut stmt = self.conn.prepare(
            r#"
            SELECT process_name, SUM(duration_seconds) as total
            FROM daily_summary
            WHERE date >= ?1 AND date <= ?2
            GROUP BY process_name
            "#,
        )?;

        let mut category_totals: std::collections::HashMap<String, i64> =
            std::collections::HashMap::new();
        let mut results = stmt.query(params![month_start, month_end])?;

        while let Some(row) = results.next()? {
            let process_name: String = row.get(0)?;
            let duration: i64 = row.get(1)?;
            let category = Self::classify_process(&process_name, &category_map);
            *category_totals.entry(category).or_insert(0) += duration;
        }

        let mut stats: Vec<CategoryStats> = category_totals
            .into_iter()
            .map(|(category_name, duration_seconds)| CategoryStats {
                category_name,
                duration_seconds,
            })
            .collect();
        stats.sort_by(|a, b| b.duration_seconds.cmp(&a.duration_seconds));
        Ok(stats)
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

    pub fn generate_missing_daily_summaries(&self, days: i64) -> Result<()> {
        let today = Local::now().date_naive();
        for i in 1..=days {
            let date = today - chrono::Duration::days(i);
            let date_str = date.format("%Y-%m-%d").to_string();
            let mut stmt = self.conn.prepare(
                "SELECT COUNT(*) FROM daily_summary WHERE date = ?1",
            )?;
            let count: i64 = stmt.query_row(params![date_str], |row| row.get(0))?;
            if count == 0 {
                let mut check_stmt = self.conn.prepare(
                    "SELECT COUNT(*) FROM usage_records WHERE start_time >= ?1 AND start_time <= ?2",
                )?;
                let start = date.and_hms_opt(0, 0, 0).unwrap();
                let end = date.and_hms_opt(23, 59, 59).unwrap();
                let start_str = Self::format_datetime(&start);
                let end_str = Self::format_datetime(&end);
                let has_data: i64 = check_stmt.query_row(params![start_str, end_str], |row| row.get(0))?;
                if has_data > 0 {
                    self.generate_daily_summary(&date)?;
                    eprintln!("[Database] Generated missing summary for {}", date_str);
                }
            }
        }
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

        let results = stmt.query_map(params![limit], |row| {
            Ok(UsageRecord {
                id: row.get(0)?,
                process_name: row.get(1)?,
                window_title: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                duration_seconds: row.get(5)?,
            })
        })?;

        results.collect()
    }

    pub fn get_daily_total(&self, date: &str) -> Result<i64> {
        let date_obj = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| {
            rusqlite::Error::InvalidParameterName("Invalid date format".to_string())
        })?;
        let start_of_day = date_obj.and_hms_opt(0, 0, 0).unwrap();
        let end_of_day = date_obj.and_hms_opt(23, 59, 59).unwrap();

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

    pub fn get_daily_top_apps(&self, date: &str, limit: usize) -> Result<Vec<(String, i64)>> {
        let date_obj = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| {
            rusqlite::Error::InvalidParameterName("Invalid date format".to_string())
        })?;
        let start_of_day = date_obj.and_hms_opt(0, 0, 0).unwrap();
        let end_of_day = date_obj.and_hms_opt(23, 59, 59).unwrap();

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

        let results =
            stmt.query_map(params![start_str, end_str, limit], |row| Ok((row.get(0)?, row.get(1)?)))?;

        results.collect()
    }

    pub fn get_daily_all_apps(&self, date: &str) -> Result<Vec<(String, i64)>> {
        let date_obj = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| {
            rusqlite::Error::InvalidParameterName("Invalid date format".to_string())
        })?;
        let start_of_day = date_obj.and_hms_opt(0, 0, 0).unwrap();
        let end_of_day = date_obj.and_hms_opt(23, 59, 59).unwrap();

        let start_str = Self::format_datetime(&start_of_day);
        let end_str = Self::format_datetime(&end_of_day);

        let mut stmt = self.conn.prepare(
            r#"
            SELECT process_name, SUM(duration_seconds) as total_duration
            FROM usage_records
            WHERE start_time >= ?1 AND start_time <= ?2
            GROUP BY process_name
            ORDER BY total_duration DESC
            "#,
        )?;

        let results = stmt.query_map(params![start_str, end_str], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;

        results.collect()
    }

    pub fn get_current_week_stats(&self) -> Result<WeekStats> {
        let today = Local::now().date_naive();
        let days_since_monday = today.weekday().num_days_from_monday() as i64;
        let week_start = today - chrono::Duration::days(days_since_monday);
        let week_end = week_start + chrono::Duration::days(6);

        let total = self.get_date_range_total(
            week_start.format("%Y-%m-%d").to_string(),
            week_end.format("%Y-%m-%d").to_string(),
        )?;

        Ok(WeekStats {
            week_start: week_start.format("%Y-%m-%d").to_string(),
            week_end: week_end.format("%Y-%m-%d").to_string(),
            total_duration: total,
        })
    }

    pub fn get_previous_week_stats(&self) -> Result<WeekStats> {
        let today = Local::now().date_naive();
        let days_since_monday = today.weekday().num_days_from_monday() as i64;
        let current_week_start = today - chrono::Duration::days(days_since_monday);
        let previous_week_start = current_week_start - chrono::Duration::days(7);
        let previous_week_end = previous_week_start + chrono::Duration::days(6);

        let total = self.get_date_range_total(
            previous_week_start.format("%Y-%m-%d").to_string(),
            previous_week_end.format("%Y-%m-%d").to_string(),
        )?;

        Ok(WeekStats {
            week_start: previous_week_start.format("%Y-%m-%d").to_string(),
            week_end: previous_week_end.format("%Y-%m-%d").to_string(),
            total_duration: total,
        })
    }

    pub fn get_week_top_apps(
        &self,
        week_start: &str,
        week_end: &str,
        limit: usize,
    ) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT process_name, SUM(duration_seconds) as total_duration
            FROM daily_summary
            WHERE date >= ?1 AND date <= ?2
            GROUP BY process_name
            ORDER BY total_duration DESC
            LIMIT ?3
            "#,
        )?;

        let results =
            stmt.query_map(params![week_start, week_end, limit], |row| Ok((row.get(0)?, row.get(1)?)))?;

        results.collect()
    }

    pub fn get_week_daily_stats(&self, week_start: &str, week_end: &str) -> Result<Vec<DateStats>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT date, SUM(duration_seconds) as total_duration
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

    pub fn get_current_month_stats(&self) -> Result<DateStats> {
        let today = Local::now().date_naive();
        let first_day_of_month = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
        let last_day_of_month = if today.month() == 12 {
            NaiveDate::from_ymd_opt(today.year() + 1, 1, 1).unwrap() - chrono::Duration::days(1)
        } else {
            NaiveDate::from_ymd_opt(today.year(), today.month() + 1, 1).unwrap()
                - chrono::Duration::days(1)
        };

        let total = self.get_date_range_total(
            first_day_of_month.format("%Y-%m-%d").to_string(),
            last_day_of_month.format("%Y-%m-%d").to_string(),
        )?;

        Ok(DateStats {
            date: first_day_of_month.format("%Y-%m").to_string(),
            total_duration: total,
        })
    }

    pub fn get_previous_month_stats(&self) -> Result<DateStats> {
        let today = Local::now().date_naive();
        let first_day_of_current_month =
            NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
        let last_day_of_previous_month = first_day_of_current_month - chrono::Duration::days(1);
        let first_day_of_previous_month = NaiveDate::from_ymd_opt(
            last_day_of_previous_month.year(),
            last_day_of_previous_month.month(),
            1,
        )
        .unwrap();

        let total = self.get_date_range_total(
            first_day_of_previous_month.format("%Y-%m-%d").to_string(),
            last_day_of_previous_month.format("%Y-%m-%d").to_string(),
        )?;

        Ok(DateStats {
            date: first_day_of_previous_month.format("%Y-%m").to_string(),
            total_duration: total,
        })
    }

    pub fn get_month_top_apps(
        &self,
        month_start: &str,
        month_end: &str,
        limit: usize,
    ) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT process_name, SUM(duration_seconds) as total_duration
            FROM daily_summary
            WHERE date >= ?1 AND date <= ?2
            GROUP BY process_name
            ORDER BY total_duration DESC
            LIMIT ?3
            "#,
        )?;

        let results =
            stmt.query_map(params![month_start, month_end, limit], |row| Ok((row.get(0)?, row.get(1)?)))?;

        results.collect()
    }

    pub fn get_month_daily_stats(&self, month_start: &str, month_end: &str) -> Result<Vec<DateStats>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT date, SUM(duration_seconds) as total_duration
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

    pub fn get_date_range_stats(&self, start_date: &str, end_date: &str) -> Result<Vec<DateStats>> {
        let (start_dt, end_dt) = self.get_range_start_end_dt(start_date, end_date)?;
        let start_str = Self::format_datetime(&start_dt);
        let end_str = Self::format_datetime(&end_dt);

        let mut stmt = self.conn.prepare(
            r#"
            SELECT date(start_time) as d, COALESCE(SUM(duration_seconds), 0) as total_duration
            FROM usage_records
            WHERE start_time >= ?1 AND start_time <= ?2
            GROUP BY date(start_time)
            ORDER BY d ASC
            "#,
        )?;

        let results = stmt.query_map(params![start_str, end_str], |row| {
            Ok(DateStats {
                date: row.get(0)?,
                total_duration: row.get(1)?,
            })
        })?;

        results.collect()
    }

    fn get_date_range_total(&self, start_date: String, end_date: String) -> Result<i64> {
        let (start_dt, end_dt) = self.get_range_start_end_dt(&start_date, &end_date)?;
        let start_str = Self::format_datetime(&start_dt);
        let end_str = Self::format_datetime(&end_dt);

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

    /// 将 "YYYY-MM-DD" 形式的起止日期转换为精确的 NaiveDateTime 区间：
    /// - start：当天 00:00:00
    /// - end：如果 end_date == 今天，则截止到**当前时间**；否则截止到当天 23:59:59
    /// 这样"生成今日报告"时能准确输出从当日 0:00 到当前时间的工作日志。
    fn get_range_start_end_dt(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<(NaiveDateTime, NaiveDateTime)> {
        let start_obj = NaiveDate::parse_from_str(start_date, "%Y-%m-%d").map_err(|_| {
            rusqlite::Error::InvalidParameterName("Invalid start date".to_string())
        })?;
        let end_obj = NaiveDate::parse_from_str(end_date, "%Y-%m-%d").map_err(|_| {
            rusqlite::Error::InvalidParameterName("Invalid end date".to_string())
        })?;

        let start_dt = start_obj
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| rusqlite::Error::InvalidParameterName("Invalid start datetime".to_string()))?;

        let today = Local::now().date_naive();
        let end_dt = if end_obj == today {
            Local::now().naive_local()
        } else {
            end_obj
                .and_hms_opt(23, 59, 59)
                .ok_or_else(|| rusqlite::Error::InvalidParameterName("Invalid end datetime".to_string()))?
        };

        // 防止用户输入 start > end 时出现反向区间，兜底裁剪
        let end_dt = end_dt.max(start_dt);

        Ok((start_dt, end_dt))
    }

    fn format_datetime(dt: &NaiveDateTime) -> String {
        dt.format("%Y-%m-%dT%H:%M:%S").to_string()
    }

    pub fn get_weekly_hourly_heatmap(&self, days: i64) -> Result<Vec<HourlyHeatmapEntry>> {
        let today = Local::now().date_naive();
        let mut entries = Vec::new();

        for day_offset in 0..days {
            let date = today - chrono::Duration::days(day_offset);
            let date_str = date.format("%Y-%m-%d").to_string();

            for hour in 0..24 {
                let start_time = date.and_hms_opt(hour, 0, 0).unwrap();
                let end_time = date.and_hms_opt(hour, 59, 59).unwrap();

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
                entries.push(HourlyHeatmapEntry {
                    date: date_str.clone(),
                    hour: hour as i32,
                    duration_seconds: duration,
                });
            }
        }

        Ok(entries)
    }

    pub fn get_hourly_heatmap_for_range(&self, start_date: &str, end_date: &str) -> Result<Vec<HourlyHeatmapEntry>> {
        let (start_dt, end_dt) = self.get_range_start_end_dt(start_date, end_date)?;
        let start_str = Self::format_datetime(&start_dt);
        let end_str = Self::format_datetime(&end_dt);

        let mut stmt = self.conn.prepare(
            r#"
            SELECT date(start_time) as date,
                   CAST(strftime('%H', start_time) AS INTEGER) as hour,
                   SUM(duration_seconds) as duration_seconds
            FROM usage_records
            WHERE start_time >= ?1 AND start_time <= ?2
            GROUP BY date(start_time), strftime('%H', start_time)
            ORDER BY date(start_time) ASC, strftime('%H', start_time) ASC
            "#,
        )?;

        let results = stmt.query_map(params![start_str, end_str], |row| {
            Ok(HourlyHeatmapEntry {
                date: row.get(0)?,
                hour: row.get(1)?,
                duration_seconds: row.get(2)?,
            })
        })?;

        results.collect()
    }

    pub fn _get_update_info(&self) -> Result<(Option<String>, Option<String>)> {
        let result: Result<(Option<String>, Option<String>), _> = self.conn.query_row(
            "SELECT skip_version, last_check FROM update_info WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );

        match result {
            Ok(info) => Ok(info),
            Err(_) => Ok((None, None)),
        }
    }

    pub fn get_records_for_date_range(&self, start_date: &str, end_date: &str, limit: usize) -> Result<Vec<UsageRecord>> {
        let (start_dt, end_dt) = self.get_range_start_end_dt(start_date, end_date)?;
        let start_str = Self::format_datetime(&start_dt);
        let end_str = Self::format_datetime(&end_dt);

        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, process_name, window_title, start_time, end_time, duration_seconds
            FROM usage_records
            WHERE start_time >= ?1 AND start_time <= ?2 AND duration_seconds >= 10
            ORDER BY duration_seconds DESC
            LIMIT ?3
            "#,
        )?;

        let results = stmt.query_map(params![start_str, end_str, limit], |row| {
            Ok(UsageRecord {
                id: row.get(0)?,
                process_name: row.get(1)?,
                window_title: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                duration_seconds: row.get(5)?,
            })
        })?;

        results.collect()
    }

    /// 按精确 ISO 时间范围查询会话明细（不裁剪到天，不过滤时长，按 start_time 升序）。
    /// start_iso / end_iso 形如 2025-08-20T09:12:33。
    pub fn get_records_between_datetimes(
        &self,
        start_iso: &str,
        end_iso: &str,
        limit: usize,
    ) -> Result<Vec<UsageRecord>> {
        let start_str = start_iso.trim().to_string();
        let end_str = end_iso.trim().to_string();
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, process_name, window_title, start_time, end_time, duration_seconds
            FROM usage_records
            WHERE start_time <= ?2 AND end_time >= ?1
            ORDER BY start_time ASC
            LIMIT ?3
            "#,
        )?;
        let results = stmt.query_map(params![start_str, end_str, limit], |row| {
            Ok(UsageRecord {
                id: row.get(0)?,
                process_name: row.get(1)?,
                window_title: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                duration_seconds: row.get(5)?,
            })
        })?;
        results.collect()
    }

    /// 在精确 ISO 时间范围内，按进程名汇总使用时长（无需 AI，纯 SQL 聚合）。
    /// 返回 [(process_name, duration_seconds)]，按时长降序。
    pub fn get_app_usage_between_datetimes(
        &self,
        start_iso: &str,
        end_iso: &str,
    ) -> Result<Vec<(String, i64)>> {
        let start_str = start_iso.trim().to_string();
        let end_str = end_iso.trim().to_string();
        let mut stmt = self.conn.prepare(
            r#"
            SELECT process_name,
                   SUM(
                     MAX(0,
                       MIN(strftime('%s', end_time),   strftime('%s', ?2))
                     - MAX(strftime('%s', start_time), strftime('%s', ?1))
                     )
                   ) AS overlap
            FROM usage_records
            WHERE start_time <= ?2 AND end_time >= ?1
            GROUP BY process_name
            HAVING overlap > 0
            ORDER BY overlap DESC
            "#,
        )?;
        let rows = stmt.query_map(params![start_str, end_str], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        rows.collect()
    }

    pub fn get_range_category_stats(&self, start_date: &str, end_date: &str) -> Result<Vec<CategoryStats>> {
        let (start_dt, end_dt) = self.get_range_start_end_dt(start_date, end_date)?;
        let start_str = Self::format_datetime(&start_dt);
        let end_str = Self::format_datetime(&end_dt);
        let category_map = self.get_app_category_map();

        let mut stmt = self.conn.prepare(
            r#"
            SELECT process_name, SUM(duration_seconds) as total
            FROM usage_records
            WHERE start_time >= ?1 AND start_time <= ?2
            GROUP BY process_name
            "#,
        )?;

        let mut category_totals: std::collections::HashMap<String, i64> =
            std::collections::HashMap::new();
        let mut results = stmt.query(params![start_str, end_str])?;

        while let Some(row) = results.next()? {
            let process_name: String = row.get(0)?;
            let duration: i64 = row.get(1)?;
            let category = Self::classify_process(&process_name, &category_map);
            *category_totals.entry(category).or_insert(0) += duration;
        }

        let mut stats: Vec<CategoryStats> = category_totals
            .into_iter()
            .map(|(category_name, duration_seconds)| CategoryStats {
                category_name,
                duration_seconds,
            })
            .collect();
        stats.sort_by(|a, b| b.duration_seconds.cmp(&a.duration_seconds));
        Ok(stats)
    }

    pub fn get_range_top_apps(&self, start_date: &str, end_date: &str, limit: usize) -> Result<Vec<(String, i64)>> {
        let (start_dt, end_dt) = self.get_range_start_end_dt(start_date, end_date)?;
        let start_str = Self::format_datetime(&start_dt);
        let end_str = Self::format_datetime(&end_dt);

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

    /// 按应用聚合：返回总时长 + 出现次数（record_count）。
    /// 与 get_range_top_apps 不同的是多了"出现次数"，避免模型自行编造频率数据。
    pub fn get_range_app_stats(
        &self,
        start_date: &str,
        end_date: &str,
        limit: usize,
    ) -> Result<Vec<AppAggStat>> {
        let (start_dt, end_dt) = self.get_range_start_end_dt(start_date, end_date)?;
        let start_str = Self::format_datetime(&start_dt);
        let end_str = Self::format_datetime(&end_dt);

        let mut stmt = self.conn.prepare(
            r#"
            SELECT process_name,
                   SUM(duration_seconds) as total_duration,
                   COUNT(*) as record_count
            FROM usage_records
            WHERE start_time >= ?1 AND start_time <= ?2
            GROUP BY process_name
            ORDER BY total_duration DESC
            LIMIT ?3
            "#,
        )?;

        let results = stmt.query_map(params![start_str, end_str, limit], |row| {
            Ok(AppAggStat {
                process_name: row.get(0)?,
                total_seconds: row.get(1)?,
                record_count: row.get(2)?,
            })
        })?;

        results.collect()
    }

    pub fn get_range_total(&self, start_date: &str, end_date: &str) -> Result<i64> {
        self.get_date_range_total(start_date.to_string(), end_date.to_string())
    }

    pub fn _set_skip_version(&self, version: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE update_info SET skip_version = ?1 WHERE id = 1",
            params![version],
        )?;
        Ok(())
    }

    pub fn _set_last_check_time(&self, time: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE update_info SET last_check = ?1 WHERE id = 1",
            params![time],
        )?;
        Ok(())
    }

    pub fn get_overrides_for_range(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<std::collections::HashMap<i64, (String, String)>> {
        let start_obj = NaiveDate::parse_from_str(start_date, "%Y-%m-%d").map_err(|_| {
            rusqlite::Error::InvalidParameterName("Invalid start date".to_string())
        })?;
        let end_obj = NaiveDate::parse_from_str(end_date, "%Y-%m-%d").map_err(|_| {
            rusqlite::Error::InvalidParameterName("Invalid end date".to_string())
        })?;
        let start_dt = start_obj.and_hms_opt(0, 0, 0).unwrap();
        let end_dt = end_obj.and_hms_opt(23, 59, 59).unwrap();
        let start_str = Self::format_datetime(&start_dt);
        let end_str = Self::format_datetime(&end_dt);

        let mut stmt = self.conn.prepare(
            r#"
            SELECT o.record_id, o.project, o.source
            FROM record_project_overrides o
            INNER JOIN usage_records r ON r.id = o.record_id
            WHERE r.start_time >= ?1 AND r.start_time <= ?2
            "#,
        )?;

        let mut map = std::collections::HashMap::new();
        let mut results = stmt.query(params![start_str, end_str])?;
        while let Some(row) = results.next()? {
            let id: i64 = row.get(0)?;
            let project: String = row.get(1)?;
            let source: String = row.get(2)?;
            map.insert(id, (project, source));
        }
        Ok(map)
    }

    pub fn set_record_override(&self, record_id: i64, project: &str, source: &str) -> Result<()> {
        let now = Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        self.conn.execute(
            r#"
            INSERT INTO record_project_overrides (record_id, project, source, updated_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(record_id) DO UPDATE SET
                project = excluded.project,
                source  = CASE WHEN excluded.source = 'user' THEN 'user' ELSE excluded.source END,
                updated_at = excluded.updated_at
            "#,
            params![record_id, project, source, now],
        )?;
        Ok(())
    }

    pub fn clear_record_override(&self, record_id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM record_project_overrides WHERE record_id = ?1",
            params![record_id],
        )?;
        Ok(())
    }

    pub fn get_config_value(&self, key: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT value FROM config WHERE key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .ok()
    }

    pub fn get_records_for_range_ordered(&self, start_date: &str, end_date: &str) -> Result<Vec<UsageRecord>> {
        let start_obj = NaiveDate::parse_from_str(start_date, "%Y-%m-%d").map_err(|_| {
            rusqlite::Error::InvalidParameterName("Invalid start date".to_string())
        })?;
        let end_obj = NaiveDate::parse_from_str(end_date, "%Y-%m-%d").map_err(|_| {
            rusqlite::Error::InvalidParameterName("Invalid end date".to_string())
        })?;
        let start_dt = start_obj.and_hms_opt(0, 0, 0).unwrap();
        let end_dt = end_obj.and_hms_opt(23, 59, 59).unwrap();
        let start_str = Self::format_datetime(&start_dt);
        let end_str = Self::format_datetime(&end_dt);

        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, process_name, window_title, start_time, end_time, duration_seconds
            FROM usage_records
            WHERE start_time >= ?1 AND start_time <= ?2
            ORDER BY start_time ASC
            "#,
        )?;

        let results = stmt.query_map(params![start_str, end_str], |row| {
            Ok(UsageRecord {
                id: row.get(0)?,
                process_name: row.get(1)?,
                window_title: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                duration_seconds: row.get(5)?,
            })
        })?;

        results.collect()
    }

    pub fn insert_or_update_report(
        &self,
        report_type: &str,
        periodicity: &str,
        start_date: &str,
        end_date: &str,
        title: &str,
        content_md: &str,
    ) -> Result<i64> {
        let tx = self.conn.unchecked_transaction()?;

        // 查已存在（按 report_type + start_date + end_date 唯一）
        let existing: Option<i64> = tx
            .query_row(
                "SELECT id FROM reports
                 WHERE report_type = ?1 AND start_date = ?2 AND end_date = ?3",
                params![report_type, start_date, end_date],
                |row| row.get(0),
            )
            .ok();

        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let id = match existing {
            Some(id) => {
                tx.execute(
                    "UPDATE reports SET
                        periodicity = ?1,
                        title       = ?2,
                        content_md  = ?3,
                        updated_at  = ?4
                     WHERE id = ?5",
                    params![periodicity, title, content_md, now, id],
                )?;
                id
            }
            None => {
                tx.execute(
                    "INSERT INTO reports
                        (report_type, periodicity, start_date, end_date, title, content_md, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                    params![report_type, periodicity, start_date, end_date, title, content_md, now],
                )?;
                tx.last_insert_rowid()
            }
        };

        tx.commit()?;
        Ok(id)
    }

    pub fn get_report(&self, id: i64) -> Result<Option<Report>> {
        let result = self.conn.query_row(
            "SELECT id, report_type, periodicity, start_date, end_date, title, content_md, created_at, updated_at
             FROM reports WHERE id = ?1",
            params![id],
            |row| {
                Ok(Report {
                    id: row.get(0)?,
                    report_type: row.get(1)?,
                    periodicity: row.get(2)?,
                    start_date: row.get(3)?,
                    end_date: row.get(4)?,
                    title: row.get(5)?,
                    content_md: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            },
        );
        match result {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn list_reports(
        &self,
        keyword: &str,
        filter_type: &str,
        filter_period: &str,
        page: i64,
        page_size: i64,
    ) -> Result<ReportListResult> {
        let page = page.max(1);
        let page_size = page_size.clamp(1, 100);
        let offset = (page - 1) * page_size;

        let kw_like = format!("%{}%", keyword);

        // 动态 WHERE 子句与参数（rusqlite 0.29 不支持 params_from_iter，
        // 用 Vec<&dyn ToSql> + .as_slice() 切片绑定，Params trait 对 &[&T] 有实现）
        let mut conditions: Vec<&str> = Vec::new();
        let mut filter_params: Vec<&dyn rusqlite::ToSql> = Vec::new();
        if !keyword.is_empty() {
            conditions.push("(title LIKE ? OR content_md LIKE ?)");
            filter_params.push(&kw_like);
            filter_params.push(&kw_like);
        }
        if !filter_type.is_empty() {
            conditions.push("report_type = ?");
            filter_params.push(&filter_type);
        }
        if !filter_period.is_empty() {
            conditions.push("periodicity = ?");
            filter_params.push(&filter_period);
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // items 查询参数 = filter 参数 + LIMIT + OFFSET
        let mut items_params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(filter_params.len() + 2);
        items_params.extend_from_slice(&filter_params);
        items_params.push(&page_size);
        items_params.push(&offset);

        let sql_items = format!(
            "SELECT id, report_type, periodicity, start_date, end_date, title, created_at, updated_at
             FROM reports {}
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?",
            where_clause
        );

        let mut stmt = self.conn.prepare(&sql_items)?;
        let items: Vec<ReportListItem> = stmt
            .query_map(items_params.as_slice(), |row| {
                Ok(ReportListItem {
                    id: row.get(0)?,
                    report_type: row.get(1)?,
                    periodicity: row.get(2)?,
                    start_date: row.get(3)?,
                    end_date: row.get(4)?,
                    title: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // total 查询（仅 filter 参数，不含 LIMIT/OFFSET）
        let sql_total = format!("SELECT COUNT(*) FROM reports {}", where_clause);
        let total: i64 =
            self.conn
                .query_row(&sql_total, filter_params.as_slice(), |row| row.get(0))?;

        Ok(ReportListResult { items, total })
    }

    pub fn delete_report(&self, id: i64) -> Result<bool> {
        let affected = self.conn.execute("DELETE FROM reports WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    pub fn export_report_to_file(&self, id: i64, file_path: &str) -> Result<bool> {
        let content_md: String = match self.conn.query_row(
            "SELECT content_md FROM reports WHERE id = ?1",
            params![id],
            |row| row.get(0),
        ) {
            Ok(c) => c,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(false),
            Err(e) => return Err(e),
        };
        std::fs::write(file_path, content_md)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        Ok(true)
    }

    // ======================================================================
    // 存储占用统计 & 过期数据清理
    // ======================================================================

    fn db_path() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ScreenTime")
            .join("screen_time.db")
    }

    /// 查询每张表的行数；返回 [(table_name, row_count)]。
    fn get_table_row_counts(&self) -> Result<Vec<(String, i64)>> {
        let tables = [
            "usage_records",
            "daily_summary",
            "reports",
            "app_categories",
            "categories",
            "record_project_overrides",
            "config",
        ];
        let mut out = Vec::with_capacity(tables.len());
        for t in tables {
            let count: i64 = self.conn.query_row(
                &format!("SELECT COUNT(*) FROM {}", t),
                [],
                |row| row.get(0),
            ).unwrap_or(0);
            out.push((t.to_string(), count));
        }
        Ok(out)
    }

    /// 查 usage_records 最早/最晚的 start_time（YYYY-MM-DD），没有则返回 None。
    fn get_usage_records_date_range(&self) -> Result<(Option<String>, Option<String>)> {
        let min: Option<String> = self.conn.query_row(
            "SELECT MIN(substr(start_time, 1, 10)) FROM usage_records",
            [],
            |row| row.get(0),
        ).unwrap_or(None);
        let max: Option<String> = self.conn.query_row(
            "SELECT MAX(substr(start_time, 1, 10)) FROM usage_records",
            [],
            |row| row.get(0),
        ).unwrap_or(None);
        Ok((min, max))
    }

    /// 统计「N 天之前的过期记录数」（分别统计 usage_records 与 daily_summary）。
    /// `days_ago` = 10 表示今天往前数第 10 天之前（不含第 10 天当天）。
    fn count_old_records(&self, days_ago: i64) -> Result<(i64, i64)> {
        let cutoff = Local::now().date_naive() - ChronoDur::days(std::cmp::max(0, days_ago - 1));
        // 2026-08-11 23:59:59 即早于 2026-08-12 的记录，即 "10 天之前"（今天 - 10 天 + 1天前一天的 24:00）
        // 这里用严格小于 cutoff_date_ymd 就可以了，因为我们是用 DATE(start_time) < cutoff_date。
        let cutoff_str = cutoff.format("%Y-%m-%d").to_string();
        let usage: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM usage_records WHERE substr(start_time, 1, 10) < ?1",
            params![cutoff_str],
            |row| row.get(0),
        ).unwrap_or(0);
        let daily: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM daily_summary WHERE date < ?1",
            params![cutoff_str],
            |row| row.get(0),
        ).unwrap_or(0);
        Ok((usage, daily))
    }

    /// 清理 days_ago 天之前的记录：
    /// - DELETE usage_records WHERE start_time DATE < cutoff
    /// - DELETE daily_summary WHERE date < cutoff
    /// 返回 (deleted_usage_count, deleted_daily_count)
    fn delete_records_older_than_days(&self, days_ago: i64) -> Result<(i64, i64)> {
        let cutoff = Local::now().date_naive() - ChronoDur::days(std::cmp::max(0, days_ago - 1));
        let cutoff_str = cutoff.format("%Y-%m-%d").to_string();

        let deleted_usage = self.conn.execute(
            "DELETE FROM usage_records WHERE substr(start_time, 1, 10) < ?1",
            params![cutoff_str],
        ).unwrap_or(0) as i64;

        let deleted_daily = self.conn.execute(
            "DELETE FROM daily_summary WHERE date < ?1",
            params![cutoff_str],
        ).unwrap_or(0) as i64;

        // 删除后尝试释放数据库文件空间（把空闲页还给文件系统）。
        // 失败不影响清理结果，忽略即可。
        let _ = self.conn.execute_batch("VACUUM; PRAGMA wal_checkpoint(TRUNCATE);");

        Ok((deleted_usage, deleted_daily))
    }
}

/// 资源占用统计（按 JSON 返回给前端）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub db_file_bytes: i64,
    pub db_file_path: String,
    pub table_rows: Vec<(String, i64)>,
    pub earliest_record_date: Option<String>,
    pub latest_record_date: Option<String>,
    pub cleanup_cutoff_days: i64,
    pub cleanup_cutoff_date: String,
    pub cleanup_usage_rows: i64,
    pub cleanup_daily_rows: i64,
}

impl Database {
    pub fn get_storage_stats(&self, cleanup_cutoff_days: i64) -> Result<StorageStats> {
        let path = Self::db_path();
        let db_file_bytes = std::fs::metadata(&path).map(|m| m.len() as i64).unwrap_or(0);
        let table_rows = self.get_table_row_counts().unwrap_or_default();
        let (earliest_record_date, latest_record_date) = self.get_usage_records_date_range().unwrap_or((None, None));

        let safe_days = std::cmp::max(1, cleanup_cutoff_days);
        let cutoff = Local::now().date_naive() - ChronoDur::days(safe_days - 1);
        let cleanup_cutoff_date = cutoff.format("%Y-%m-%d").to_string();
        let (cleanup_usage_rows, cleanup_daily_rows) = self.count_old_records(safe_days).unwrap_or((0, 0));

        Ok(StorageStats {
            db_file_bytes,
            db_file_path: path.to_string_lossy().to_string(),
            table_rows,
            earliest_record_date,
            latest_record_date,
            cleanup_cutoff_days: safe_days,
            cleanup_cutoff_date,
            cleanup_usage_rows,
            cleanup_daily_rows,
        })
    }

    /// 清理 N 天前的过期数据；返回 CleanupResult。
    pub fn cleanup_expired_records(&self, older_than_days: i64) -> Result<CleanupResult> {
        let before = {
            let path = Self::db_path();
            std::fs::metadata(&path).map(|m| m.len() as i64).unwrap_or(0)
        };
        let (deleted_usage, deleted_daily) = self.delete_records_older_than_days(older_than_days)?;
        // 再执行一次 VACUUM wal_checkpoint(TRUNCATE) 确保文件收缩
        let _ = self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        // Windows 下 VACUUM 需要没有其他读事务；失败不影响主流程。
        let _ = self.conn.execute_batch("VACUUM;");
        let after = {
            let path = Self::db_path();
            std::fs::metadata(&path).map(|m| m.len() as i64).unwrap_or(0)
        };
        // 回收 WAL/SHM 临时文件大小
        let path = Self::db_path();
        let freed_wal: i64 = ["-wal", "-shm"].iter().filter_map(|suf| {
            let p = path.with_extension(format!("db{}", suf));
            match std::fs::metadata(&p) {
                Ok(m) => Some(m.len() as i64),
                Err(_) => {
                    // 清理前可能有，但检查点之后就删了；返回 0 即可
                    None
                }
            }
        }).sum();
        let saved_bytes = std::cmp::max(0, before - after) + freed_wal;
        let safe_days = std::cmp::max(1, older_than_days);
        let cutoff = Local::now().date_naive() - ChronoDur::days(safe_days - 1);
        Ok(CleanupResult {
            cutoff_date: cutoff.format("%Y-%m-%d").to_string(),
            cutoff_days: safe_days,
            deleted_usage_rows: deleted_usage,
            deleted_daily_rows: deleted_daily,
            db_size_before_bytes: before,
            db_size_after_bytes: after,
            saved_bytes,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupResult {
    pub cutoff_date: String,
    pub cutoff_days: i64,
    pub deleted_usage_rows: i64,
    pub deleted_daily_rows: i64,
    pub db_size_before_bytes: i64,
    pub db_size_after_bytes: i64,
    pub saved_bytes: i64,
}
