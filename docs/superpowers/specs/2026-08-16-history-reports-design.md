# 历史报告模块设计文档

- 日期：2026-08-16
- 模块：历史报告（History Reports）
- 路由：`/history-reports`
- 关联页面：[HistoryReports.tsx](file:///c:/Repo/Code/ScreenManager/src/pages/HistoryReports.tsx)（当前为空占位）
- 已有能力：[ollama.rs](file:///c:/Repo/Code/ScreenManager/src-tauri/src/ollama.rs) 中 `generate_report` 同步生成 Markdown（5 种模板 + 本地 Ollama qwen3:4b）

## 1. 背景与目标

### 1.1 背景

TraeWork 已通过 Ollama 链同步生成日报/周报/月报（standard/tech/project/concise/pomodoro 共 5 种模板），但**生成结果不落库**，也没有列表页可供回看、搜索、再次导出。用户希望"日报周报月报助手"的核心闭环是：生成 → 存档 → 查看 → 搜索 → 复用。

### 1.2 目标

- **生成**：历史报告页内嵌新建入口（模板 + 日期范围），一站式生成报告并自动保存
- **列表**：按分页展示历史报告，支持模板/周期筛选 + 模糊搜索
- **查看**：点击报告条目弹出 Markdown 预览面板
- **管理**：6 个动作——查看预览、复制内容、导出 .md、删除、重新生成、搜索
- **升级路径**：数据模型为未来的异步生成任务留足空间（不改表即可升级为异步）

### 1.3 非目标（YAGNI）

- 不做异步生成/任务队列/调度器（本次用同步，未来升级不动 schema 即可加）
- 不做 .docx/.pdf 导出（只做 .md）
- 不做报告分享（生成链接）
- 不做报告间 diff/compare
- 不新增 markdown 渲染依赖（package.json 无 react-markdown/markdown-it，用轻量自写解析器）
- 不做批量操作（批量删除、批量导出）——可单条执行

## 2. 关键决策

| 维度 | 决策 | 理由 |
|---|---|---|
| 页面定位 | 列表 + 新建入口合页 | 一站式体验，不跨页跳 |
| 存储 | DB 存正文（reports 表） | 查询/分页/搜索方便，同库同生命周期 |
| 分类 | 模板名（report_type）+ 周期（periodicity）2 维 | 独立筛选，表达力强 |
| 管理动作 | 查看、复制、导出.md、删除、重新生成、搜索 | 用户全选，覆盖完整闭环 |
| 生成方式 | 同步生成 + 立即入库（方案 A） | 不改 Ollama 链路，改动最小可控 |
| Markdown 渲染 | 轻量自写解析器（不加新依赖） | 生成格式固定可控，包体积无膨胀 |
| 双轨过渡 | 不双轨，直接改 `generate_report` 签名 | 不维护两套逻辑 |

## 3. 架构与数据流

```
┌──────── 生成流程 ────────┐
│ 用户点「生成报告」        │
│  ↓                       │
│ Tauri: create_and_save_report(type, start, end)
│    ↓                    │
│    ollama.rs: generate_report ── Ollama 推理(阻塞)
│      ↓ 成功              │
│      DB: insert_or_update_report → 返回 report_id
│      ↓                   │
│    返回 { report_id, content }
│  ↓                       │
│  前端：刷新列表 + 自动弹出预览面板
└──────────────────────────┘

┌──────── 列表/查看流程 ────┐
│ 加载页面                  │
│   ↓ list_reports 分页查询  │
│ 展示卡片列表 + 分页条       │
│   ↓ 点卡片「查看」         │
│   get_report(id) → 取正文  │
│   ↓                      │
│  右弹预览面板：Markdown 渲染+工具条(复制/导出/删除/重新生成)
└──────────────────────────┘

┌──────── 搜索/筛选流程 ────┐
│ 搜索框输入/筛选下拉变更     │
│  ↓ list_reports(keyword,   │
│    filter_type, filter_period,
│    page, page_size)        │
│  WHERE title LIKE ? OR content LIKE ?
│  + 类型/周期过滤 + LIMIT/OFFSET
│  ↓ 返回 { items, total }  │
│ 列表刷新 + 分页条重新计算   │
└──────────────────────────┘
```

资源消耗特征：
- 生成：同步等待 Ollama 推理 20-60s（CPU 占用高，但与当前行为一致）
- 列表：每次查询 20 条，不含 content_md 大字段，毫秒级
- 搜索：LIKE 扫 content_md 几千字，报告总量在百级/千级时无压力

## 4. 数据模型：schema v4 → v5

### 4.1 迁移脚本

```sql
CREATE TABLE reports (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    report_type TEXT    NOT NULL,           -- standard / tech / project / concise / pomodoro
    periodicity TEXT    NOT NULL,           -- daily / weekly / monthly
    start_date  TEXT    NOT NULL,           -- YYYY-MM-DD
    end_date    TEXT    NOT NULL,           -- YYYY-MM-DD
    title       TEXT    NOT NULL,           -- 列表显示标题（生成时拼）
    content_md  TEXT    NOT NULL,           -- Markdown 正文（几千字）
    created_at  TEXT    NOT NULL,           -- 初次生成时间
    updated_at  TEXT    NOT NULL            -- 重新生成会更新
);
CREATE INDEX idx_reports_created ON reports(created_at DESC);
CREATE INDEX idx_reports_type    ON reports(report_type, periodicity);
```

`database.rs`：
- `CURRENT_SCHEMA_VERSION`: 4 → 5
- `run_migrations` 追加 `from_version<5` 分支 → 调 `migration_v5()` + 插 schema_version 记录
- 新增 `migration_v5()`：建表 + 建索引

### 4.2 periodicity 判定规则（前后端一致）

```
days = diff_days(end_date, start_date) + 1
days == 1 → daily
1 < days <= 31 → weekly
days > 31 → monthly
```

两端都实现同一套判定，避免不一致。后端以 DB 写入时的判定为准。

### 4.3 title 生成模板

列表展示的标题（两端都实现，最终写库以 ollama.rs 为准）：

```
daily:   "标准日报 · 2026-08-16"
weekly:  "技术周报 · 2026-08-10 ~ 2026-08-16"
monthly: "项目月报 · 2026-08"
```

### 4.4 重新生成的 UPSERT 语义

`insert_or_update_report` 按唯一键 `(report_type, start_date, end_date)` 查：
- 找到已存在 → `UPDATE content_md, title, updated_at = now WHERE id = ?` → 返回同 id
- 未找到 → `INSERT` → 返回新 id

这样"重新生成"按钮永远是幂等的：点 N 次也不产生重复记录，只是 updated_at 变新。

SQLite 没有原生 upsert 可以不用 INSERT ... ON CONFLICT。用"先选再判"的显式事务即可，简单清晰。

## 5. 后端设计

### 5.1 Database 新方法

在 [database.rs](file:///c:/Repo/Code/ScreenManager/src-tauri/src/database.rs) 追加：

```rust
// 写库：重新生成 UPSERT，返回 report_id
pub fn insert_or_update_report(
    &self,
    report_type: &str,
    periodicity: &str,
    start_date: &str,
    end_date: &str,
    title: &str,
    content_md: &str,
) -> Result<i64>;

// 单条详情（含 content_md），用于预览
pub fn get_report(&self, id: i64) -> Result<Option<Report>>;

// 列表（不含 content_md，取 20 条快）
pub fn list_reports(
    &self,
    keyword: &str,        // "" = 不搜
    filter_type: &str,    // "" = 不过滤
    filter_period: &str,  // "" = 不过滤
    page: i64,            // 从 1 开始
    page_size: i64,       // 默认 20
) -> Result<(Vec<ReportListItem>, i64)>;

// 删除
pub fn delete_report(&self, id: i64) -> Result<bool>;

// 导出 .md 文件
pub fn export_report_to_file(&self, id: i64, file_path: &str) -> Result<bool>;
```

辅助结构（放在 database.rs `#[derive(...)]` 区）：

```rust
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
```

**`list_reports` SQL 要点**：
- `items`：SELECT id/report_type/periodicity/start_date/end_date/title/created_at/updated_at（不要 content_md！）
- `keyword != ""` 时加 `WHERE (title LIKE ?1 OR content_md LIKE ?1)`——注意 content_md 虽然被搜到，但 SELECT 列表里**不取回**（仅用于过滤，节省传输）
- 再附加 AND 条件：`filter_type != "" → report_type = ?X`、`filter_period != "" → periodicity = ?X`
- ORDER BY created_at DESC
- LIMIT page_size OFFSET (page-1)*page_size
- `total`：同样条件下 `SELECT COUNT(*)`，独立跑一次

### 5.2 Ollama 生成链改造

**[ollama.rs](file:///c:/Repo/Code/ScreenManager/src-tauri/src/ollama.rs)**：

原 `generate_report(db, params) -> Result<String, String>`（只返回正文）。改为：

```rust
pub async fn generate_and_save_report(
    db: &Arc<Mutex<Database>>,
    params: GenerateReportParams,
) -> Result<(i64, String), String> {
    // 1. 取数据（与原 generate_report 完全相同的 DB 查询段）
    let data = { /* 原 ReportData 组装 */ };
    if data.total_duration == 0 {
        return Err("所选时间范围内没有工作数据".to_string());
    }

    // 2. 算 periodicity + title
    let periodicity = {
        let days = days_between(&params.start_date, &params.end_date) + 1;
        if days <= 1 { "daily" } else if days <= 31 { "weekly" } else { "monthly" }
    };
    let type_name = match params.report_type.as_str() {
        "standard" => "标准",
        "tech" => "技术",
        "project" => "项目",
        "concise" => "简洁",
        "pomodoro" => "番茄钟",
        _ => "工作",
    };
    let title = if periodicity == "daily" {
        format!("{}日报 · {}", type_name, params.start_date)
    } else if periodicity == "weekly" {
        format!("{}周报 · {} ~ {}", type_name, params.start_date, params.end_date)
    } else {
        format!("{}月报 · {}-{}", type_name, ...params.start_date.slice(0,7)...)
        // 实际写法：format!("{}月报 · {}", type_name, &params.start_date[..7])
    };

    // 3. 原 Ollama 推理调用（完全照搬，不改动 prompt/模型/超时）
    let context = build_context(&data);
    let prompt = build_prompt(&params.report_type, &context);
    // ... 原 reqwest 调用 ...
    let content_md = ollama_resp.response;

    // 4. 写库 → 拿 id
    let id = {
        let dbg = db.lock().map_err(|e| format!("DB lock error: {}", e))?;
        dbg.insert_or_update_report(
            &params.report_type,
            periodicity,
            &params.start_date,
            &params.end_date,
            &title,
            &content_md,
        ).map_err(|e| format!("保存报告失败: {}", e))?
    };

    Ok((id, content_md))
}
```

要点：
- 函数名**改为** `generate_and_save_report`（旧 `generate_report` 在改完所有调用点后**删除**，不双轨）
- 新增辅助函数 `days_between(a, b)`（chrono 解析求天数）
- 原 `ReportData`/`build_context`/`build_prompt` 完全不动，只在末尾加一步 DB 写入

### 5.3 Tauri commands 注册

**[main.rs](file:///c:/Repo/Code/ScreenManager/src-tauri/src/main.rs)** 旧的 `generate_report` command 删除，替换为：

```rust
#[tauri::command]
async fn create_and_save_report(
    state: tauri::State<'_, Arc<Mutex<MonitorState>>>,
    report_type: String,
    start_date: String,
    end_date: String,
) -> Result<(i64, String), String> {
    let params = ollama::GenerateReportParams { report_type, start_date, end_date };
    // state 中 db 是 Arc<Mutex<Database>>，clone 一份给 async
    let db = state.lock().unwrap().db.clone();
    ollama::generate_and_save_report(&db, params).await
}
```

追加 4 个同步 command：

```rust
#[tauri::command]
fn list_reports(
    state: tauri::State<'_, Arc<Mutex<MonitorState>>>,
    keyword: String,
    filter_type: String,
    filter_period: String,
    page: i64,
    page_size: i64,
) -> (Vec<database::ReportListItem>, i64) { /* db.list_reports(...).unwrap_or_default() */ }

#[tauri::command]
fn get_report(state: ..., id: i64) -> Option<database::Report> { /* ... */ }

#[tauri::command]
fn delete_report(state: ..., id: i64) -> bool { /* ... */ }

#[tauri::command]
fn export_report_to_file(state: ..., id: i64, file_path: String) -> bool { /* ... */ }
```

`tauri::generate_handler!`：删除旧的 `generate_report`，追加 5 个新 handler。

## 6. 前端设计

### 6.1 API 封装

[api.ts](file:///c:/Repo/Code/ScreenManager/src/utils/api.ts)：

删除旧的 `generateReport` 方法（与后端双改同步），新增：

```typescript
export interface ReportListItem {
  id: number
  report_type: string
  periodicity: 'daily' | 'weekly' | 'monthly'
  start_date: string
  end_date: string
  title: string
  created_at: string
  updated_at: string
}

export interface Report extends ReportListItem {
  content_md: string
}

export interface ReportListResult {
  items: ReportListItem[]
  total: number
}

// api 对象内新增：
createAndSaveReport: (reportType: string, startDate: string, endDate: string) =>
  invoke<[number, string]>('create_and_save_report', { reportType, startDate, endDate }),

listReports: (keyword: string, filterType: string, filterPeriod: string, page: number, pageSize: number) =>
  invoke<{ items: ReportListItem[], total: number }>('list_reports', { keyword, filterType, filterPeriod, page, pageSize }),

getReport: (id: number) =>
  invoke<Report | null>('get_report', { id }),

deleteReport: (id: number) =>
  invoke<boolean>('delete_report', { id }),

exportReportToFile: (id: number, filePath: string) =>
  invoke<boolean>('export_report_to_file', { id, filePath }),
```

注意：Rust 返回 `(Vec<ReportListItem>, i64)`（tuple）在前端 serialize 为 `[items, total]` 的数组也可能是 `{ 0: items, 1: total }`——为避免歧义，**Rust 侧 `list_reports` command 把返回类型改为结构化对象**：

```rust
#[derive(Serialize)]
struct ReportListResult {
    items: Vec<ReportListItem>,
    total: i64,
}
```

统一为 `{ items, total }` 对象，前后端清晰无坑。

### 6.2 页面结构：HistoryReports.tsx 完全重写

页面分为 4 个区块：

```
[新建报告卡片]
  模板：5 个模板按钮（标准/技术/项目/简洁/番茄钟）
  范围：预设档（近1天 / 近7天 / 近30天 / 本月 / 上月）+ 自定义起□至□
  「生成报告」按钮 → loading（禁用+转圈） → 成功后自动刷新列表 + 打开预览面板

[筛选与搜索]
  搜索框（placeholder: "搜索报告标题或内容..."）+ debounced onChange 触发查询
  模板筛选下拉 [全部/标准/技术/项目/简洁/番茄钟]
  周期筛选下拉 [全部/日报/周报/月报]

[报告卡片列表]
  卡片 grid 或 list 布局，每卡显示：
    title + 范围（start~end）
    标签：[周期：日/周/月] · [模板类型] ·  updated_at 相对时间（如"2小时前"）
    操作按钮：[查看] [复制] [导出.md] [重新生成] [删除]
  空态："暂无报告，试试生成第一份~"

[分页条]
  上一页 / 页码 1 2 3 4... / 下一页，共 N 条 M 页
  默认 pageSize = 20

[预览面板（点击查看/生成成功自动打开）]
  右侧抽屉或模态，固定宽 60% 或 max-900px
  顶部工具条：
    标题（title）+ [关闭]
    [复制全文] [导出.md] [删除] [重新生成]
  中部：Markdown 渲染区（轻量自写解析器，详见 6.3）
```

**用户动作处理细节**：
- **生成报告**：loading 期间禁止重复点击；失败弹 error toast（沿用 Ollama 链路的错误文案）；成功后 `listReports` 重新拉一次 + `getReport(id)` 打开预览
- **查看**：点卡片或「查看」按钮 → `getReport(id)` → 打开面板
- **复制全文**：用 `navigator.clipboard.writeText(content_md)`，成功后按钮变「已复制 ✔」3 秒
- **导出.md**：用 `@tauri-apps/plugin-dialog` 的 `save` dialog，默认文件名 = `title.replace(/[:/]/g,'-') + '.md'`，拿到路径后调 `api.exportReportToFile(id, path)`；成功 toast "已导出到..."
- **删除**：弹出确认窗（"删除报告 XXXX？不可恢复。"），确认后 `deleteReport(id)` → 从 items 中移除本地条目 + 关面板（如果开着）
- **重新生成**：确认窗（"用相同参数重新生成？会覆盖旧内容。"）→ 调用 `createAndSaveReport(原report_type, 原start, 原end)`（因为后端按 upsert，返回同 id）→ 完成后刷新内容（`getReport(id)` 拿新内容）+ 刷新列表 updated_at

### 6.3 Markdown 轻量渲染器（无新依赖）

报告由自己的 prompt 生成，格式固定有限。写一个 60-80 行的 `renderMarkdownLite(md: string): JSX.Element[]` 纯函数，支持：

- 标题：`# H1 / ## H2 / ### H3` → `<h1/h2/h3>`
- 有序/无序列表：`- item` / `1. item` → `<ul/ol><li>`
- **粗体**：`**text**` → `<strong>`
- *斜体*：`*text*` → `<em>`
- 段落：空行分隔 → `<p>`
- 代码块：`` ``` ``` `` → `<pre><code>`（如有）
- 表格：不支持（自己的 prompt 不生成表格）

用正则分段解析，逐段处理。不处理嵌套 markdown（如列表项里的粗体也能顺带识别）。输出 React 节点列表。页面直接渲染：

```tsx
<div className="md-preview">
  {renderMarkdownLite(report.content_md)}
</div>
```

样式：`.md-preview h1/h2/h3/p/ul/ol/li/pre/code/strong/em` 定义在 `HistoryReports.css`。

### 6.4 HistoryReports.css 新建

- 新建报告卡片：`.hr-new-card / .hr-template-row / .hr-range-row / .preset-btn / .custom-range / .hr-generate-btn`
- 筛选条：`.hr-filter-bar / .hr-search / .hr-select`
- 列表：`.hr-list / .hr-report-card / .hr-card-title / .hr-card-meta / .hr-card-tags / .hr-tag / .hr-actions`
- 分页：`.hr-pagination / .hr-page-btn / .hr-page-info`
- 预览面板：`.hr-preview-drawer / .hr-preview-header / .hr-preview-tools / .hr-preview-body`
- Markdown 预览：`.md-preview h1 / h2 / h3 / p / ul / ol / li / pre / code / strong / em / blockquote`

风格沿用 TodayWork / WorkTimeline 的卡片风格（圆角 16、边框 1、`--card-bg`、`--border-color`、`--theme-color`）。

### 6.5 Tauri 文件保存对话框依赖

前端调用 `save` dialog 需 `@tauri-apps/plugin-dialog`，检查 package.json 是否已有：
- 已有 → 直接用 `import { save } from '@tauri-apps/plugin-dialog'`
- 未装 → 装：`pnpm add @tauri-apps/plugin-dialog`

这是 Tauri 官方插件，项目中其他地方（如导出设置/导入设置）大概率已用——先查，已有就直接用，未装才加依赖。

## 7. 前后端组件清单

### 7.1 Rust

**database.rs**：
- `CURRENT_SCHEMA_VERSION` 4→5
- `run_migrations` 追加 `from_version<5` 分支
- 新增 `migration_v5()` 建 reports 表 + 2 索引
- 新增 struct `Report` / `ReportListItem` / `ReportListResult`
- 新增 5 个 Database 方法：`insert_or_update_report` / `get_report` / `list_reports` / `delete_report` / `export_report_to_file`

**ollama.rs**：
- 函数 `generate_report` → 改名 + 增强为 `generate_and_save_report`，返回 `(id, content_md)`
- 保留原 build_context / build_prompt / ReportData / GenerateReportParams，完全不动
- 新增 `days_between` 辅助函数
- 在末尾加一步：写库拿 id

**main.rs**：
- 旧 `generate_report` command 移除
- 新增 5 个 commands：`create_and_save_report`（async）/ `list_reports` / `get_report` / `delete_report` / `export_report_to_file`
- generate_handler! 同步增删

### 7.2 前端

**api.ts**：
- 删除旧 `generateReport` 方法
- 新增 `Report` / `ReportListItem` / `ReportListResult` 接口
- 新增 5 个 invoke 方法

**HistoryReports.tsx**（完全重写）：
- 新建面板（模板 + 范围预设/自定义 + 生成按钮）
- 筛选 + 搜索（debounce）
- 卡片列表（6 动作按钮）
- 分页
- 预览面板（抽屉）+ 顶部 4 个工具按钮
- Markdown 轻量渲染器 `renderMarkdownLite`

**HistoryReports.css**（新建）：
- 全部上面 6.4 节列出的 class
- md-preview 子样式

## 8. 错误处理

| 场景 | 处理 |
|---|---|
| Ollama 未启动 / 连接失败 | 沿用原错误提示 toast，不写库 |
| 模型未下载 / qwen3:4b 不存在 | 沿用 Ollama response 错误 |
| 范围无工作数据 | 返回 "没有工作数据" 错误，不写库 |
| 生成成功但写 DB 失败 | 前端特殊处理：错误中含"保存报告失败"时，自动把正文复制到剪贴板，提示"生成成功但保存失败，内容已复制" |
| 搜索 DB 出错（如 content_md 列被误删）| list_reports 返回空数组，不抛异常；前端提示"加载失败" |
| get_report 查不到（已被删）| 返回 null，前端提示"报告不存在或已删除" |
| delete_report 失败 | 返回 false，前端提示"删除失败，请重试" |
| export_report_to_file 路径无权限 | 返回 false，前端提示"写入失败，请换路径" |
| 重新生成期间 Ollama 失败 | 保留旧内容不覆盖，前端提示"重新生成失败，旧内容仍在" |
| 分页参数 <1 或过大 | 后端 page<1 则 page=1；page_size>100 则=100，防滥用 |

## 9. 测试

**Rust（手动跑）**：
- `insert_or_update_report`：先 INSERT → 再同参数 UPSERT → id 不变，updated_at 变
- `list_reports`：空关键词全量、关键词命中 title/content、过滤类型+周期、分页计数
- `delete_report`：存在删得 true，不存在返回 false
- `export_report_to_file`：文件能被写出到临时路径
- `cargo test --lib` 已有的 10 个 session_aggregator 测试保持通过

**前端（手动验证）**：
- 生成报告（standard 模板 + 今天）→ 成功 + 写库 + 列表出现 + 预览对
- 同参数"重新生成"→ id 不变，updated_at 更新，内容可能变化（正常）
- 不同参数 → 产生新条目，新旧都在
- 筛选/搜索各档独立与组合命中正确
- 翻页：50 条数据，20/页，3 页正确
- 复制全文 → 剪贴板内容 = content_md
- 导出.md → 文件内容 = content_md
- 删除 → 条目消失，预览面板关闭（如开着）
- Markdown 渲染：标题/列表/粗体/段落均正确

## 10. 实现顺序建议

1. 后端 schema v5 migration + Database 6 个新方法（database.rs，含 derive structs）
2. Ollama 链改造（ollama.rs：改名+写库+periodicity/title）
3. Tauri commands 注册（main.rs：删除旧 generate_report，新增 5 个）
4. 前端 api.ts 增删接口
5. HistoryReports.css 新建（先写好样式骨架）
6. HistoryReports.tsx 重写（UI + 数据流 + 6 动作 + Markdown 渲染器）
7. 端到端验证：生成→列出→查看→4动作→重新生成→删除

## 11. 未来扩展点（本次不做）

- 异步任务队列/生成状态机（schema 不动，新增 report_tasks 表）
- .docx/.pdf 导出
- 报告分享链接
- 批量删除/批量导出
- 报告 diff / compare
- 报告内插图（从截图附件生成）
- 完整 Markdown 渲染（加 react-markdown 依赖，替换 renderMarkdownLite）
