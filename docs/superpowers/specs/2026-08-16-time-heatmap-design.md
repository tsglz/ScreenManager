# 时段热力图模块设计文档

- 日期：2026-08-16
- 模块：时段热力图（Time Heatmap）
- 路由：`/time-heatmap`
- 关联页面：[TimeHeatmap.tsx](file:///c:/Repo/Code/ScreenManager/src/pages/TimeHeatmap.tsx)（当前为空占位）
- 参考样式：[TodayWork.tsx 时段记录区](file:///c:/Repo/Code/ScreenManager/src/pages/TodayWork.tsx#L169-L224) 的 6 级色阶网格

## 1. 背景与目标

### 1.1 背景

项目转型为日报/周报/月报输出助手。今日工作页的"时段记录"是固定近 7 天 × 24 小时的热力图，无法查看任意历史时段。时段热力图模块负责补齐这一环：用户选定任意日期范围后，按合适的维度展示该范围的活动强度分布，并支持下钻查看明细。

### 1.2 目标

- 支持预设档（近7天/近30天/本月/上月）与自定义日期范围
- 按范围天数自动切换网格维度，保证可读性
- hover 显示单元格时长，点击下钻到工作时间线查看会话明细
- 与 TodayWork 的时段记录保持视觉一致

### 1.3 非目标（YAGNI）

- 不做按分类着色（聚合后"主导分类"语义弱，且 TodayWork 是单色）
- 不做自定义色阶
- 不做热力图导出
- 不做大于 365 天的范围（前端限制）
- 不做后端维度参数化（维度切换纯前端聚合）

## 2. 关键决策

| 维度 | 决策 | 理由 |
|---|---|---|
| 时段选择 | 预设档 + 自定义日期范围 | 兼顾快速与灵活 |
| 网格维度 | 按范围切换（≤7天/8-31天/>31天） | 多天时行数多，单一维度不可读 |
| 交互 | hover tooltip + 点击下钻 | 复用已有 WorkTimeline 模块，不重复造轮子 |
| 计算时机 | 后端返原始数据，前端按维度聚合（方案 A） | 后端一个查询，维度切换不重请求，体验流畅 |

## 3. 架构与数据流

```
用户选日期范围
    ↓
Rust: getHourlyHeatmapForRange(start, end) → Vec<{date, hour, duration}>
    ↓
前端按当前维度聚合（aggregateByDimension）
    ↓
网格渲染（复用 TodayWork 的 .heat-cell / .level-0..5）
    ↓
点击单元格 → navigate('/work-timeline?start=...&end=...')
```

数据流特征：

1. 用户选范围（预设档点击 / 自定义日期变更）
2. 前端调用 `api.getHourlyHeatmapForRange(start, end)`
3. Rust 侧一次 SQL 聚合返回原始 `{date, hour, duration}` 数组
4. 前端按当前维度（由范围天数自动判定）二次聚合为网格数据
5. 渲染网格，hover 显示 tooltip，点击下钻

资源特征：30 天 = 720 条记录，90 天 = 2160 条，均为 KB 级，传输无压力。维度切换是纯前端状态变更，不重新请求后端。

## 4. 后端设计

### 4.1 新增 Database 方法

在 [database.rs](file:///c:/Repo/Code/ScreenManager/src-tauri/src/database.rs#L1270) 紧随 `get_weekly_hourly_heatmap` 之后新增：

```rust
pub fn get_hourly_heatmap_for_range(
    &self,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<HourlyHeatmapEntry>> {
    // 解析 start_date/end_date（YYYY-MM-DD）→ start_dt(00:00:00) / end_dt(23:59:59)
    // SQL：
    //   SELECT date(start_time) as date,
    //          CAST(strftime('%H', start_time) AS INTEGER) as hour,
    //          SUM(duration_seconds) as duration
    //   FROM usage_records
    //   WHERE start_time >= ?1 AND start_time <= ?2
    //   GROUP BY date, hour
    //   ORDER BY date ASC, hour ASC
    // 返回 Vec<HourlyHeatmapEntry>（复用现有 struct）
}
```

与现有 `get_weekly_hourly_heatmap` 的差异：

| 项 | `get_weekly_hourly_heatmap` | `get_hourly_heatmap_for_range` |
|---|---|---|
| 参数 | `days: i64`（从今天往前） | `start_date, end_date`（任意范围） |
| 实现 | 逐天循环 × 24 小时共 N×24 次查询 | 一次 SQL GROUP BY 聚合 |
| 返回 | 含 0 时长空格 | 仅返回非空小时 |

一次 GROUP BY 比 N×24 次循环更高效，且支持任意历史区间。

### 4.2 新增 Tauri command

在 [main.rs](file:///c:/Repo/Code/ScreenManager/src-tauri/src/main.rs) 新增：

```rust
#[tauri::command]
fn get_hourly_heatmap_for_range(
    state: tauri::State<'_, Arc<Mutex<MonitorState>>>,
    start_date: String,
    end_date: String,
) -> Vec<HourlyHeatmapEntry> {
    state
        .lock()
        .unwrap()
        .db
        .lock()
        .unwrap()
        .get_hourly_heatmap_for_range(&start_date, &end_date)
        .unwrap_or_default()
}
```

在 `tauri::generate_handler!` 列表追加 `get_hourly_heatmap_for_range`。

## 5. 前端设计

### 5.1 API 封装

在 [api.ts](file:///c:/Repo/Code/ScreenManager/src/utils/api.ts) 的 `getWeeklyHourlyHeatmap` 之后追加：

```typescript
getHourlyHeatmapForRange: (startDate: string, endDate: string) =>
  invoke<HourlyHeatmapEntry[]>('get_hourly_heatmap_for_range', { startDate, endDate }),
```

复用已有的 `HourlyHeatmapEntry` 接口（`{ date: string, hour: number, duration_seconds: number }`）。

### 5.2 维度切换规则

按所选范围天数（`end - start + 1`）自动判定：

| 范围天数 | 维度 | 行 | 列 | 列数 |
|---|---|---|---|---|
| `days <= 7` | `day` | 每天 | 小时（0-23） | 24 |
| `8 <= days <= 31` | `week` | ISO 周（范围内涉及的周） | 小时（0-23） | 24 |
| `days > 31` | `month` | 月（范围内涉及的月） | 日（1-31） | 31 |

维度由范围自动决定，不暴露给用户切换。范围改变后维度可能变，网格结构跟着变。

### 5.3 前端聚合函数

纯函数 `aggregateByDimension(rawData, dimension, rangeStart, rangeEnd) -> GridData`：

```typescript
type Dimension = 'day' | 'week' | 'month'

interface GridData {
  rowLabels: string[]      // 行标签（如 "08/16 周三"、"第33周"、"2026-08"）
  colCount: number         // 列数（24 或 31）
  colLabels: string[]      // 列标签（稀疏标注，如 ["0:00","","","3:00",...]）
  cells: number[][]        // cells[rowIdx][colIdx] = duration_seconds
  rowDrillDown: { start: string; end: string }[]  // 每行对应的下钻日期范围
  cellDrillDown: { start: string; end: string }[][] // 每格对应的下钻日期范围
  maxDuration: number      // 用于色阶计算
}
```

聚合逻辑：

- `day`：按 `entry.date` 分组为行，`entry.hour` 为列。行标签=`MM/DD 周X`，行下钻=start=end=该天。
- `week`：用 JS Date 算 ISO 周键（`"YYYY-Wxx"`），按周键分组为行，`entry.hour` 为列。每格=该周内该小时的总时长。行标签=`第XX周`，行下钻=该周一到周日。
- `month`：按 `entry.date.slice(0,7)`（YYYY-MM）分组为行，`parseInt(entry.date.slice(8,10))` 为列。每格=该月内该日的总时长。行标签=`YYYY-MM`，行下钻=该月初到月末。

ISO 周算法：用 `Date` 算 `getWeek()`（非标准但主流浏览器支持）或手写 ISO 8601 周计算。初版用手写函数避免兼容性争议。

### 5.4 网格样式

复用 TodayWork 的 CSS class，保持视觉一致：

- `.heatmap-container` / `.heatmap-scroll` / `.heatmap-grid` / `.hm-corner` / `.hm-hour-label` / `.hm-date-label` / `.heat-cell` / `.level-0..5` / `.heatmap-legend` / `.legend-cell`

这些 class 定义在 [TodayWork.css](file:///c:/Repo/Code/ScreenManager/src/pages/TodayWork.css)。TimeHeatmap 新建独立 CSS 文件 `TimeHeatmap.css`，但热力图相关 class 直接复用 TodayWork.css 的定义（通过 `@import` 或在 TimeHeatmap.css 中重新声明同一套样式）。

**决策**：在 TimeHeatmap.css 中重新声明热力图相关 class（不 `@import`），避免 CSS 加载顺序依赖。代码有少量重复（约 50 行），但模块独立、可单独维护。TodayWork.css 的热力图样式不动。

列标签稀疏规则：

- `day` / `week`：每 3 小时标（0:00 / 3:00 / 6:00 / ... / 21:00），其余空。与 TodayWork 一致。
- `month`：每 5 天标（1 / 5 / 10 / 15 / 20 / 25 / 30），其余空。

### 5.5 色阶

与 TodayWork 完全一致的 6 级色阶（`level-0` 到 `level-5`），按单元格时长 / 网格最大时长的比例分级：

```typescript
function getHeatLevel(duration: number, max: number): number {
  if (max === 0 || duration === 0) return 0
  const ratio = duration / max
  if (ratio <= 0.15) return 1
  if (ratio <= 0.35) return 2
  if (ratio <= 0.55) return 3
  if (ratio <= 0.75) return 4
  return 5
}
```

`max` 取当前网格内所有单元格的最大时长。维度切换后重新计算 max。

### 5.6 范围选择 UI

```
[近7天] [近30天] [本月] [上月] [自定义]
                                  ↑ 选中后展开：
                                  起 [date input] 至 [date input]
                                  最大值=今天
```

- 预设档 4 个按钮，点击即切，自动算出 start/end
- 自定义档选中后展开两个 date input
- 范围显示：`2026-08-01 ~ 2026-08-16 · day 维度`（显示当前范围与自动判定的维度名）
- 范围校验：start > end 时禁用查询并提示；范围 > 365 天时提示并截断

### 5.7 交互：hover + 点击下钻

**hover**：原生 `title` 属性显示 tooltip，内容格式：

- `day`：`2026-08-16 14:00 — 1h 23m`
- `week`：`第33周 14:00 — 5h 12m`（该周该小时累计）
- `month`：`2026-08 16日 — 3h 45m`（该月该日累计）

无活动显示「无活动」。

**点击下钻**：点击单元格 → `navigate('/work-timeline?start={start}&end={end}')`

下钻粒度按维度：

| 维度 | 单元格语义 | 下钻 start/end |
|---|---|---|
| `day` | 某天某小时 | start=end=该天日期（整天） |
| `week` | 某周某小时 | start=该周一日期，end=该周日日期（整周） |
| `month` | 某月某日 | start=该月初，end=该月末（整月） |

下钻到天/周/月而非精确到小时，因为 WorkTimeline 的 `get_work_sessions(start, end)` 只支持连续日期范围。用户在 WorkTimeline 里看该范围的会话明细，会话本身就带小时精度，不丢信息。

## 6. WorkTimeline 增强：读 URL 参数

为支持下钻，WorkTimeline 需适配 URL search params：

- 读 `?start=...&end=...`
- 若存在且合法，初始化 `customStart`/`customEnd` 并切到 `custom` 模式
- 若不存在或非法，保持原默认行为（今天）

实现：用 React Router 的 `useSearchParams` hook。在 [WorkTimeline.tsx](file:///c:/Repo/Code/ScreenManager/src/pages/WorkTimeline.tsx) 的 `range` useMemo 中加入 URL 参数判断。

改动范围小（约 10 行），不破坏现有功能。

## 7. 前后端组件清单

### 7.1 Rust 后端

**[database.rs](file:///c:/Repo/Code/ScreenManager/src-tauri/src/database.rs)**：

- `get_hourly_heatmap_for_range(start_date, end_date) -> Result<Vec<HourlyHeatmapEntry>>`（一次 SQL GROUP BY）

**[main.rs](file:///c:/Repo/Code/ScreenManager/src-tauri/src/main.rs)**：

- Tauri command `get_hourly_heatmap_for_range(start_date, end_date)`
- 注册进 `generate_handler!`

### 7.2 前端

**[api.ts](file:///c:/Repo/Code/ScreenManager/src/utils/api.ts)**：

- `getHourlyHeatmapForRange(start, end)` 封装

**[TimeHeatmap.tsx](file:///c:/Repo/Code/ScreenManager/src/pages/TimeHeatmap.tsx)** 重写：

- 范围选择（预设档 + 自定义）
- 维度自动判定
- `aggregateByDimension` 聚合函数
- 网格渲染（复用 TodayWork 的 heatmap CSS class）
- hover tooltip + 点击下钻

**`src/pages/TimeHeatmap.css`** 新建：

- 热力图相关 class（复用 TodayWork.css 的样式定义）
- 范围选择器样式
- 页面布局样式

**[WorkTimeline.tsx](file:///c:/Repo/Code/ScreenManager/src/pages/WorkTimeline.tsx)** 增强：

- 读 URL `?start=...&end=...` 初始化范围

## 8. 错误处理与测试

### 8.1 错误处理

- 查询失败：前端空状态 + 错误提示
- 范围无效（start > end）：前端校验，禁用查询并提示
- 范围过大（>365 天）：前端限制，提示并截断到 365 天
- 后端返回空数组：渲染空网格（全 level-0），不报错
- URL 参数非法（下钻入口）：WorkTimeline 忽略非法参数，回退默认

### 8.2 测试

**Rust**：

- `get_hourly_heatmap_for_range` 单元测试：空范围、单天、跨月、无数据

**前端**（手动验证，项目无前端测试框架）：

- `aggregateByDimension` 三种维度的聚合正确性
- 维度自动判定按天数切换
- 范围选择器交互
- 下钻 URL 参数正确传递
- WorkTimeline 读 URL 参数初始化

## 9. 实现顺序建议

1. 后端 `get_hourly_heatmap_for_range` + Tauri command（database.rs + main.rs）
2. 前端 api.ts 封装
3. `aggregateByDimension` 聚合函数 + 维度判定
4. TimeHeatmap.tsx 重写（范围选择 + 网格 + hover + 下钻）
5. TimeHeatmap.css 新建
6. WorkTimeline.tsx 增强（读 URL 参数）
7. 端到端验证

## 10. 未来扩展点（本次不做）

- 按分类着色（每个单元格按主导分类上色）
- 自定义色阶
- 热力图导出为图片
- 维度手动切换（用户强制 day 维度看 30 天）
- 大于 365 天的范围（需后端分页或预聚合）
