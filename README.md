# Screen Manager

一款基于本地 Ollama AI 的桌面应用使用时间追踪与工作报告生成工具。自动记录前台窗口使用时长，提供多维度统计、时段热力图、工作会话分析，并支持通过本地大模型一键生成标准日报、技术日报、番茄钟报告等多种类型的工作文档。

## 功能特性

### 核心功能
- **时间追踪**：自动记录前台窗口应用的使用时长，支持空闲检测自动暂停
- **AI 工作报告**：集成本地 Ollama 大模型，一键生成多种类型的工作报告（标准日报、技术日报、项目日报、简洁日报、番茄钟聚类报告）
- **多维度统计**：支持按天、按周、按月统计应用使用时长与分类占比
- **时段热力图**：小时级活动分布热力图，可视化识别一天中的效率高峰与低谷
- **工作会话与时间线**：自动聚合连续专注时段，可视化工作时间线，支持手动标记项目归属
- **历史报告管理**：报告本地存储、搜索、筛选、查看、Markdown 导出；支持按报告类型、周期、关键词过滤
- **应用分类管理**：灵活的进程 → 分类映射，默认含开发工具、浏览器、社交通讯、游戏娱乐、办公工具、系统工具、ScreenManager 等分类
- **数据存储与清理**：可视化数据库占用统计，一键清理 N 天前的过期明细与日汇总记录（AI 报告与分类配置保留）
- **开机启动**：可配置开机自动启动，关闭窗口自动最小化到系统托盘
- **主题切换**：支持日间 / 夜间模式切换
- **隐私保护**：所有数据 100% 本地存储，不上传任何服务器；AI 报告生成仅连接本机 Ollama 服务

### 界面展示
- **今日工作**：首页实时显示当日工作概况、时长、应用排行
- **生成报告**：选择日期范围、报告类型和 Ollama 模型，AI 生成结构化工作文档
- **工作时间线**：连续专注时段可视化，手动标记/清除记录项目归属
- **时段热力图**：小时级 × 日期的二维热力图，定位效率规律
- **应用记录**：明细使用记录查询与项目标注
- **历史报告**：已生成报告的列表、搜索、预览、删除、导出 Markdown
- **隐私保护**：本地数据与隐私说明
- **设置页面**：开机自启、主题、HTTP 代理、Ollama 默认模型、存储统计与清理、版本信息、自动更新

### 系统集成
- **系统托盘**：后台静默运行，托盘菜单快捷操作
- **关闭最小化**：关闭窗口时自动最小化到托盘，不影响数据记录
- **代理配置**：支持自定义 HTTP/HTTPS 代理，保存后立即生效；并内置多个 GitHub Release 镜像加速源用于自动更新
- **Windows 原生**：基于 Windows 平台开发，系统资源占用低

## 技术栈

| 层级 | 技术 | 说明 |
|------|------|------|
| 前端框架 | React 18 + TypeScript | 用户界面 |
| 后端框架 | Tauri 2.x | 桌面应用框架 |
| 后端语言 | Rust | 系统级逻辑与窗口监控 |
| 图表库 | Recharts | 数据可视化（折线、饼图、热力图等） |
| 路由 | React Router 7 | 页面导航 |
| 数据库 | SQLite (rusqlite + bundled) | 本地数据存储 |
| 异步运行时 | Tokio | 后端异步任务调度 |
| 网络请求 | reqwest | Ollama API 与更新检查 |
| AI 报告 | Ollama (本地大模型) | 报告生成，支持 qwen3 / llama3 等 |
| 更新插件 | tauri-plugin-updater v2 | 自动检查、下载与安装更新 |
| HTTP 客户端 | reqwest (no_proxy 本地免代理) | Ollama localhost 请求强制直连 |

## 项目结构

```
screen-manager/
├── src/                              # 前端源代码
│   ├── pages/                        # 页面组件
│   │   ├── TodayWork.tsx             # 今日工作首页
│   │   ├── GenerateReport.tsx        # AI 报告生成
│   │   ├── WorkTimeline.tsx          # 工作时间线（会话可视化）
│   │   ├── TimeHeatmap.tsx           # 时段热力图
│   │   ├── AppRecords.tsx            # 应用明细记录
│   │   ├── Daily.tsx / Weekly.tsx    # 周/月统计页面
│   │   │   / Monthly.tsx
│   │   ├── HistoryReports.tsx        # 历史报告管理
│   │   ├── Privacy.tsx               # 隐私保护说明
│   │   └── Settings.tsx              # 设置页面
│   ├── components/                   # 公共组件
│   │   ├── Layout.tsx                # 整体布局（侧边栏 + 内容区）
│   │   ├── Sidebar.tsx               # 侧边导航（含主题切换）
│   │   ├── Header.tsx                # 页面头部
│   │   └── UpdateModal.tsx           # 自动更新弹窗
│   ├── hooks/                        # React Hooks
│   │   ├── useUpdater.ts             # 自动更新 Hook
│   │   └── useTheme.ts               # 主题切换 Hook
│   ├── utils/
│   │   ├── api.ts                    # Tauri invoke API 调用封装
│   │   └── format.ts                 # 时间/日期/字节格式化工具
│   ├── App.tsx                       # 路由与根组件
│   └── main.tsx                      # 入口文件
│
└── src-tauri/                        # 后端源代码 (Rust)
    ├── src/
    │   ├── main.rs                   # 程序入口 + Tauri 命令定义
    │   ├── lib.rs                    # 库入口
    │   ├── database.rs               # SQLite 数据库操作
    │   ├── window_monitor.rs         # 前台窗口监控 + 空闲检测
    │   ├── tray.rs                   # 系统托盘菜单与事件
    │   ├── scheduler.rs              # 定时任务（生成日报汇总）
    │   ├── session_aggregator.rs     # 工作会话聚合（专注时段识别）
    │   ├── ollama.rs                 # Ollama 模型列表 + AI 报告生成
    │   ├── config.rs                 # 应用配置（代理、默认模型）
    │   ├── autostart.rs              # 开机自启注册表管理
    │   └── update.rs                 # 版本号、数据目录
    ├── capabilities/                 # Tauri 权限配置
    └── tauri.conf.json               # Tauri 配置（含 updater）
```

## 开发

### 环境要求
- Node.js 18+
- Rust 1.70+
- Windows 10/11
- （可选）[Ollama](https://ollama.com/) 本地服务（用于 AI 报告生成，推荐 `qwen3:4b-fp16` 或同量级模型）

### 快速开始

```bash
# 安装依赖
npm install

# 开发模式运行（同时启动前端 + Tauri 窗口）
npm run tauri:dev
```

**启动 Ollama（如需 AI 报告功能）**：
```bash
ollama pull qwen3:4b-fp16
ollama serve
```

### 生产构建

```bash
# 构建安装包（含 MSI / NSIS 等 bundle）
npm run tauri:build
```

### 可用命令

| 命令 | 说明 |
|------|------|
| `npm run dev` | 仅启动前端 Vite 开发服务器 |
| `npm run build` | 构建前端生产版本（tsc + vite build） |
| `npm run tauri:dev` | 启动 Tauri 开发模式（前端 + Rust + 窗口） |
| `npm run tauri:build` | 构建 Tauri 生产版本（生成安装包与 updater 签名） |

## 数据存储

应用数据存储在用户本地，具体位置如下：

**存储路径**：`%APPDATA%\ScreenTime\`

**存储文件**：
| 文件 | 说明 |
|------|------|
| `screen_time.db` | SQLite 数据库，存储使用记录、日汇总、AI 报告、分类映射、项目归属、配置项 |
| `config.json` | 应用配置（HTTP 代理、Ollama 默认模型等） |
| `categories.json` | 应用分类配置 |

**重要**：更新程序时会自动保留上述所有数据文件。

**存储统计与清理**（设置页 → 数据存储与清理）：
- 可视化数据库文件大小、各表记录数、最早/最近记录日期
- 一键清理 10 天前的过期 `usage_records` / `daily_summary` 明细
- AI 报告、分类配置、手动项目归属不会被清理
- 执行前二次确认，执行后显示释放空间

## 配置与代理

### 应用配置（`AppConfig`）

存储于 `%APPDATA%\ScreenTime\config.json` 或通过设置页管理：

| 字段 | 类型 | 说明 |
|------|------|------|
| `http_proxy` | string \| null | 自定义 HTTP/HTTPS 代理，留空读系统环境变量或直连 |
| `ollama_model` | string \| null | Ollama 默认模型名，未设置时使用内置 `qwen3:4b-fp16` |

### 代理行为
- 应用启动最早阶段：读取 `config.json` 并写入进程环境变量（先于任何网络请求）
- 前端设置页保存后：立即应用到当前进程环境变量
- Ollama `localhost` 请求：强制 `no_proxy()`，避免代理拦截本机请求

## AI 工作报告

### 模型选择
Ollama 报告模型选择优先级（高 → 低）：
1. 生成时用户本次选择（GenerateReport 页下拉）
2. AppConfig 中保存的 `ollama_model`
3. 内置默认：`qwen3:4b-fp16`

### 报告类型
实际生成时会按「周期 × 模板」组合，共 5 × 3 = 15 种：周期按日期跨度自动判定（1 天 = 日报，2-7 天 = 周报，≥ 8 天 = 月报）；模板由用户在生成页下拉选择。

| 模板 | 日报 (daily) | 周报 (weekly) | 月报 (monthly) |
|------|------|------|------|
| `standard` | **标准日报**：按分类归纳完成工作，附总览 + 节奏（单行）+ 效率建议 + 明日关注 | **标准周报**：额外有「本周节奏表」（按天 Top3 应用/环比昨日/最长专注）和「环比上周对比表」；末尾「下周关注」 | **标准月报**：节奏表按日期展开，环比上月；末尾「下月调节点」 |
| `tech` | **技术日报**：总览显式显示「开发类占比」；**额外**加「开发工具分类明细」section（按开发子分类列出应用明细表）；建议聚焦 IDE 连续调试/浏览器查资料分心源等技术场景 | **技术周报**：技术日报的区块 + 本周节奏 + 环比上周 | **技术月报**：同上，换周期 |
| `project` | **项目日报**：总览显式显示「未标记项目占比 xx%」引导补归属；**额外**加「项目投入排名」表 | **项目周报**：项目日报区块 + 本周节奏 + 环比上周（含开发/分心占比对比） | **项目月报**：同上，换周期 |
| `concise` | **简洁日报**：仅总览 + 应用 Top 5 + 建议 + 明日关注（分类/时段/会话全部不渲染）；overview ≤ 80 字，建议 ≤ 2 条 | **简洁周报**：同上 + 本周节奏 + 环比，所有 Top N 缩到最小规模 | **简洁月报**：同上，换周期 |
| `pomodoro` | **番茄钟聚类报告**：不渲染应用/分类/时段，只保留「专注时段 Top 15」（专注段视角），建议围绕专注间隔/休息 | **番茄钟周报/月报**：同上 + 本周节奏 + 环比 | — |

### 智能数据维度（报告上下文包含）
所有**报告正文中的数字**（时长、百分比、排名、出现次数）全部由程序在 SQLite 中通过聚合 SQL 预先算出，**绝不交给大模型自己统计**。下面列出这些数据的权威来源与样本规模：

- 总时长 / 每日时长 → `get_range_total` + `get_date_range_stats`（SQL 聚合，不依赖样本）
- Top N 应用排行（附分类标注 / 出现次数 / 占比%）→ `get_range_app_stats` 按 `process_name` GROUP BY，按报告模板类型取前 5/10/12；权威聚合，不依赖样本
- 分类统计（占比%）→ `get_range_category_stats`（SQL GROUP BY 类别名），权威聚合
- 小时级活动分布（热力分析效率高峰）→ `get_hourly_heatmap_for_range`
- 工作会话（连续专注时段 + 打断次数 + 项目归属）→ `aggregate_sessions`（基于 idle_threshold / switch_threshold 两次拆会话；项目来自手动归属 `record_project_overrides` + IDE 窗口标题启发式，未命中归 "其他"）
- 项目投入排名 / 未标记占比 → 跨所有会话的 `projects` 切片再聚合（纯函数，不新增查询）
- 本周节奏表（按天 Top3 应用 / 环比昨日 / 最长专注）→ 按日期循环调 `get_daily_top_apps(date,3)` + 会话按日期取最长；周报/月报渲染，日报单行可选显示
- 环比上周对比表（总时长/日均/专注段数/开发占比/分心占比 5 项，带 ±%/pp 和 ↑↓）→ 把查询区间整体向前平移 day_span 天作为 prev 区间，重跑上面所有聚合函数；仅周报/月报渲染
- 活动记录**样本**（仅作为调试/参考，不在报告正文中渲染）：按周期动态限流 — 日报 120 条、周报 300 条、月报 500 条。**注意：这只是模型写作参考里附带的样本量，并不影响正文的排名/时长/占比数值（因为正文来自上面的 SQL 聚合），所以不存在「Top 50 稀释」的问题。**
- 下周关注候选 / 明日待办（outlook 候选清单）：程序根据「未标记占比高、分心占比 >10%、开发占比环比下滑、会话数下降、单日过忙」等规则生成 4-5 条候选；模型只能在此范围内写 outlook 段，不会凭空编造项目名或数字。

### 历史报告管理
- 搜索关键词 + 按报告类型 / 周期过滤 + 分页
- Markdown 预览、编辑覆盖、删除
- 导出 `.md` 文件到本地

## 工作会话与项目归属

- `session_idle_threshold`（默认 900 秒 = 15 分钟）：空闲超阈值则拆分会话
- `session_switch_threshold`（默认 300 秒 = 5 分钟）：短时切换不计为打断
- 在「工作时间线」页面可手动将单条记录 `set_record_project` 标记为指定项目
- 或 `clear_record_project` 清除手动归属
- 标记后报告中的「工作会话」段将自动按项目维度聚合展示

## 自动更新

### 配置说明
首次配置已完成，项目默认：
- **公钥**：已写入 `src-tauri/tauri.conf.json → plugins.updater.pubkey`
- **更新源**：依次尝试以下端点（含 2 个 GitHub 镜像加速，解决国内访问问题）
  1. `https://mirror.ghproxy.com/https://github.com/tsglz/ScreenManager/releases/latest/download/latest.json`
  2. `https://ghproxy.net/https://github.com/tsglz/ScreenManager/releases/latest/download/latest.json`
  3. `https://github.com/tsglz/ScreenManager/releases/latest/download/latest.json`
- **Windows 安装模式**：`passive`（无人值守被动安装）

### 更新流程
1. 应用启动后延迟 2 秒后台检查更新
2. 发现新版本时显示更新弹窗
3. 用户可选择：
   - **立即更新**：下载并安装更新（带进度回调）
   - **稍后提醒**：关闭弹窗，下次启动再提示
4. 下载完成后提示用户重启应用

### 手动检查更新
在「设置」页点击「检查更新」按钮可手动触发更新检查。

## 版本

当前版本：**2.1.0**

## 许可证

MIT License
