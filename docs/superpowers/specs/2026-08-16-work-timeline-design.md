# 工作时间线模块设计文档

- 日期：2026-08-16
- 模块：工作时间线（Work Timeline）
- 路由：`/work-timeline`
- 关联页面：[WorkTimeline.tsx](file:///c:/Repo/Code/ScreenManager/src/pages/WorkTimeline.tsx)（当前为空占位）

## 1. 背景与目标

### 1.1 背景

项目正由屏幕时间监控工具转型为日报/周报/月报输出助手。现有数据采集能力（[window_monitor.rs](file:///c:/Repo/Code/ScreenManager/src-tauri/src/window_monitor.rs) 每 100ms 轮询前台窗口，落库 `usage_records`）已完备，但缺少「把零散使用记录聚合成有语义的工作进度」的能力。工作时间线模块负责补齐这一环：给定一个时段，整理出该时段内进行了哪些工作、各占多久。

### 1.2 目标

- 在选定日期范围内，把 `usage_records` 聚合成「工作会话」序列
- 每个会话标注主导项目、起止时间、时长、子项目分布
- 支持用户事后修正单条记录的项目归属
- 资源消耗尽可能低

### 1.3 非目标（YAGNI）

- 不做截图 + AI 识别（架构预留，本次不实现）
- 不改造监控循环、不新增后台任务
- 不做项目规则的图形化编辑器（初版硬编码常见开发工具规则）
- 不做跨年超长范围查询的性能优化（限制最大范围即可）

## 2. 关键决策

| 维度 | 决策 | 理由 |
|---|---|---|
| 任务粒度 | 按项目/主题 | 贴近真实工作进度，而非单纯按应用切 |
| 项目识别 | 自动提取窗口标题 + 事后修正 | 零配置起步，逐步修正沉淀 |
| 时段切分 | 按工作会话 | 主动换项目或长空闲断界，会话内归主导项目 |
| 截图 AI | 架构预留不实现 | 重资源，本次只留字段和优先级链路 |
| 计算时机 | 查询时计算（方案 A） | 零后台开销，单日数据量小，毫秒级聚合 |

## 3. 架构与数据流

纯查询时计算，零后台任务，不触碰监控循环与 scheduler：

```
usage_records(已有) ──┐
                      ├─→ Rust: 查记录 + 查覆盖 → aggregate_sessions() ─→ Tauri command ─→ 前端时间线
record_project_overrides(新增) ─┘
```

数据流：

1. 用户打开「工作时间线」页，选定日期范围
2. 前端调用 Tauri command `get_work_sessions(start, end)`
3. Rust 侧查询该范围内的 `usage_records` 与 `record_project_overrides`
4. 新模块 `session_aggregator.rs` 在内存中聚合为 `Vec<WorkSession>`
5. 返回前端渲染

资源特征：打开页面才计算，其余时刻零开销。单日 `usage_records` 通常几十到几百条，Rust 侧聚合为毫秒级，用户无感。

## 4. 数据模型

### 4.1 Schema 迁移（v3 → v4）

新增覆盖表，**不污染原始数据**：

```sql
CREATE TABLE record_project_overrides (
    record_id  INTEGER PRIMARY KEY,           -- 关联 usage_records.id
    project    TEXT NOT NULL,                 -- 用户指定的项目名
    source     TEXT NOT NULL DEFAULT 'user',  -- 'user' | 预留 'ai'
    updated_at TEXT NOT NULL                  -- ISO 时间戳
);
```

迁移点：`CURRENT_SCHEMA_VERSION` 由 3 升至 4，在 `database.rs` 现有迁移链尾追加 v4 分支。

### 4.2 项目优先级链路

聚合时对每条记录计算 `project`，优先级：

```
用户覆盖(source='user') > 自动提取(extract_project) > "其他"
```

未来 AI 识别结果写入同一张表标 `source='ai'`，优先级低于 `user`、高于自动提取即可接入。这是「架构预留不实现」的落点：表和优先级链路就位，AI 写入路径留空，聚合逻辑零改动即可生效。

### 4.3 阈值配置

两个阈值写入 `config` 表（已有键值存储），可调：

| 键 | 默认值 | 含义 |
|---|---|---|
| `session_idle_threshold` | `900`（15 分钟，秒） | 超过此时长无活动，断会话 |
| `session_switch_threshold` | `300`（5 分钟，秒） | 短暂切到别的项目不足此时长，不视为换任务 |

## 5. 项目自动提取

Rust 侧纯函数：

```rust
fn extract_project(process_name: &str, window_title: &str) -> Option<String>;
```

针对已知开发工具按标题模式提取项目名：

| 应用 | 标题示例 | 提取结果 |
|---|---|---|
| VSCode | `main.rs - ScreenManager - Visual Studio Code` | `ScreenManager` |
| Trae CN / Cursor | `WorkTimeline.tsx - ScreenManager` | `ScreenManager` |
| IntelliJ 系 | `main.rs – ScreenManager` | `ScreenManager` |
| 浏览器 / 通讯等 | `... - Google Chrome` | `None`（归会话主导） |

规则：

- 仅对已知开发工具类 `process_name` 尝试提取，避免误判
- 提取不到的记录不强制归类，聚合时并入所在会话的主导项目
- 初版硬编码常见开发工具规则，不做图形化编辑器

## 6. 会话聚合算法

### 6.1 输入输出

输入：按 `start_time` 排序的 `Vec<UsageRecord>` + `HashMap<record_id, (project, source)>`（覆盖表）

聚合前解析覆盖：直接取 record_id 对应的 project。`record_project_overrides` 以 `record_id` 为 PRIMARY KEY，同一 record 只保留一条最高优先级覆盖——user 写入时 upsert 掉已有的 ai 覆盖（详见 4.2）。

输出：`Vec<WorkSession>`

```rust
struct WorkSession {
    start_time: String,
    end_time: String,
    total_seconds: i64,
    main_project: String,
    projects: Vec<ProjectSlice>,   // {name, seconds} 子项目时长分布
    record_count: i32,
}
```

### 6.2 算法

```
1. 预处理：每条记录算 project = override.get(id) > extract(p, t) > "其他"
2. 遍历（已按时间排序），维护 current_session：
   - gap(与前一条 end_time 的间隔) > session_idle_threshold
        → 空闲断点，结束 current_session，开新会话
   - 当前 project != None 且 != 会话主导 且 该 project 在会话内累计 > session_switch_threshold
        → 主动换项目，结束 current_session，开新会话
   - 否则
        → 并入 current_session
3. 会话结束（或遍历完）时计算：
   - main_project = 会话内按 duration 加权最多的 project
   - projects = 各 project 的 duration 分布
4. 会话内 project 为 None 的记录，其时长计入 main_project
```

### 6.3 断点判定细节

- **空闲断点**：两条记录间隔超过 `session_idle_threshold`（默认 15 分钟）。午休、开会、离开都走这条。
- **换项目断点**：某非主导项目在会话内累计时长超过 `session_switch_threshold`（默认 5 分钟）。短暂切浏览器查资料不足 5 分钟不会断段，会并入主导项目。
- 会话首条记录的 project 即初始主导；后续主导随累计时长动态更新。

## 7. 前后端组件

### 7.1 Rust 后端

**[database.rs](file:///c:/Repo/Code/ScreenManager/src-tauri/src/database.rs)**：

- schema v4 迁移：建 `record_project_overrides`，写入两个默认阈值到 `config`
- `get_overrides_for_range(start, end) -> HashMap<i64, (String, String)>`（record_id → (project, source)）
- `set_record_override(record_id, project, source)`
- `clear_record_override(record_id)`
- `get_config_value(key) -> Option<String>`（读取阈值，供聚合使用）

**新模块 `session_aggregator.rs`**：

- `extract_project(process_name, window_title) -> Option<String>`
- `aggregate_sessions(records, overrides, idle_threshold, switch_threshold) -> Vec<WorkSession>`

纯函数，无 I/O 依赖，便于单元测试。

**[main.rs](file:///c:/Repo/Code/ScreenManager/src-tauri/src/main.rs)**：

- 新增 command `get_work_sessions(start: String, end: String) -> Result<Vec<WorkSession>>`
- 新增 command `set_record_project(record_id: i64, project: String) -> Result<()>`
- 新增 command `clear_record_project(record_id: i64) -> Result<()>`
- 在 `tauri::generate_handler!` 注册上述 command

### 7.2 前端

**[api.ts](file:///c:/Repo/Code/ScreenManager/src/utils/api.ts)**：

新增封装：

```typescript
interface ProjectSlice { name: string; seconds: number }
interface WorkSession {
  start_time: string; end_time: string; total_seconds: number;
  main_project: string; projects: ProjectSlice[]; record_count: number;
}
getWorkSessions(start: string, end: string): Promise<WorkSession[]>
setRecordProject(recordId: number, project: string): Promise<void>
clearRecordProject(recordId: number): Promise<void>
```

**[WorkTimeline.tsx](file:///c:/Repo/Code/ScreenManager/src/pages/WorkTimeline.tsx)** 重写：

- 日期范围选择：默认今天，可切本周、自定义
- 纵向时间轴：每个会话一个块，显示起止时间、时长、主导项目色块、子项目分布条
- 会话块点击展开记录列表
- 单条记录可改项目归属 → 调 `setRecordProject` → 刷新会话
- 空状态复用现有 `.empty-state` 样式

## 8. 错误处理与测试

### 8.1 错误处理

- 查询/聚合失败：前端空状态 + 错误提示
- 单条记录异常（如时间格式错误）跳过，不阻断整体聚合
- 阈值缺失：用默认值兜底，不报错
- 覆盖表 record_id 找不到对应记录：忽略该覆盖

### 8.2 测试

`session_aggregator.rs` 纯函数单元测试：

- `extract_project`：各开发工具标题格式、非开发工具返回 None、空标题
- `aggregate_sessions`：
  - 单会话（无断点）
  - 空闲断点（间隔超阈值）
  - 换项目断点（累计超阈值）
  - 短暂切换不断（累计未超阈值）
  - 用户覆盖优先于自动提取
  - 空记录列表返回空会话

## 9. 实现顺序建议

1. schema v4 迁移 + 覆盖表 CRUD（database.rs）
2. `session_aggregator.rs` 纯函数 + 单元测试
3. Tauri command 注册（main.rs）
4. 前端 api.ts 封装
5. WorkTimeline.tsx 页面重写
6. 端到端验证

## 10. 未来扩展点（本次不做）

- 截图 + AI 识别：写入 `record_project_overrides` 标 `source='ai'`，按优先级链路接入
- 项目规则的图形化编辑器
- 跨年超长范围查询的分页/预聚合
- 会话维度的备注与标签
