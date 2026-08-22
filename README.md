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
| 类型 | 说明 |
|------|------|
| `standard` | 标准日报：按类别归纳已完成工作，附个性化效率分析 |
| `tech` | 技术日报：侧重 IDE / 调试 / 代码编写等技术维度 |
| `project` | 项目日报：按项目 / 工作主题组织工作内容与进展 |
| `concise` | 简洁日报：只列 3–5 项最关键工作，适合快速汇报 |
| `pomodoro` | 番茄钟聚类：按活动类型聚类成 3–6 个工作集群 |

### 智能数据维度（报告上下文包含）
- 总时长、每日时长
- Top 20 应用排行（附分类标注）
- 分类统计（占比%）
- 小时级活动分布（热力分析效率高峰）
- 工作会话（连续专注时段 + 打断次数 + 项目归属）
- Top 50 详细活动记录（每条含分类标签）
- 个性化效率分析指令（专注、分心、效率高峰、久坐提醒、具体建议）

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
