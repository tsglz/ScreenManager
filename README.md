# Screen Manager

一款简洁高效的桌面应用使用时间追踪工具，帮助用户了解自己在各个应用上的时间分布。

## 功能特性

### 核心功能
- **时间追踪**：自动记录前台窗口应用的使用时长
- **多维度统计**：支持今日、本周、本月及自定义日期范围统计
- **分类统计**：支持按应用分类（如：工作、娱乐、开发等）进行时间统计
- **开机启动**：可配置开机自动启动，最小化到系统托盘
- **自动更新**：支持自动检查和安装更新

### 界面展示
- **Dashboard**：实时显示今日使用概况，包括总时长、应用排行、分类占比
- **日报/周报/月报**：详细的时间使用报告，支持环形图和排行榜
- **分类管理**：灵活的应用分类配置
- **设置页面**：版本信息、数据路径、开机自启配置

### 系统集成
- **系统托盘**：后台静默运行，点击托盘图标可快速查看统计
- **关闭最小化**：关闭窗口时自动最小化到托盘，不影响数据记录
- **Windows 原生**：基于 Windows 平台开发，系统资源占用低

## 技术栈

| 层级 | 技术 | 说明 |
|------|------|------|
| 前端框架 | React 18 + TypeScript | 用户界面 |
| 后端框架 | Tauri 2.x | 桌面应用框架 |
| 后端语言 | Rust | 系统级逻辑 |
| 图表库 | Recharts | 数据可视化 |
| 路由 | React Router | 页面导航 |
| 数据库 | SQLite | 本地数据存储 |
| 更新插件 | tauri-plugin-updater | 自动更新 |

## 项目结构

```
screen-manager/
├── src/                      # 前端源代码
│   ├── pages/               # 页面组件
│   │   ├── Dashboard.tsx    # 主页
│   │   ├── Daily.tsx        # 日报
│   │   ├── Weekly.tsx       # 周报
│   │   ├── Monthly.tsx      # 月报
│   │   └── Settings.tsx     # 设置页面
│   ├── components/           # 公共组件
│   │   ├── UpdateModal.tsx  # 更新弹窗
│   │   └── ...
│   ├── hooks/               # React Hooks
│   │   └── useUpdater.ts    # 更新检查 Hook
│   ├── utils/
│   │   └── api.ts           # API 调用封装
│   ├── App.tsx              # 根组件
│   └── main.tsx             # 入口文件
│
└── src-tauri/               # 后端源代码 (Rust)
    ├── src/
    │   ├── main.rs          # 程序入口
    │   ├── lib.rs           # 库入口
    │   ├── database.rs      # 数据库操作
    │   ├── window_monitor.rs # 窗口监控
    │   ├── tray.rs          # 系统托盘
    │   ├── scheduler.rs     # 定时任务
    │   ├── autostart.rs     # 开机启动
    │   └── update.rs        # 更新检查
    ├── capabilities/        # 权限配置
    │   └── default.json     # 默认权限
    └── tauri.conf.json      # Tauri 配置
```

## 开发

### 环境要求
- Node.js 18+
- Rust 1.70+
- Windows 10/11

### 快速开始

```bash
# 安装依赖
npm install

# 开发模式运行
npm run tauri:dev

# 生产构建
npm run tauri:build
```

### 可用命令

| 命令 | 说明 |
|------|------|
| `npm run dev` | 启动前端开发服务器 |
| `npm run build` | 构建前端生产版本 |
| `npm run tauri:dev` | 启动 Tauri 开发模式 |
| `npm run tauri:build` | 构建 Tauri 生产版本 |

## 数据存储

应用数据存储在用户本地，具体位置如下：

**存储路径**：`%APPDATA%\ScreenTime\`

**存储文件**：
| 文件 | 说明 |
|------|------|
| `screen_time.db` | SQLite 数据库，存储使用记录 |
| `config.json` | 应用配置文件 |
| `categories.json` | 应用分类配置 |

**重要**：更新程序时会自动保留上述所有数据文件。

## 自动更新

### 配置说明

首次配置需要完成以下步骤：

1. **生成签名密钥对**
   ```bash
   cd src-tauri
   cargo tauri signer generate
   ```
   这将生成 `private.key` 和 `public.key` 文件。

2. **配置公钥**
   打开 `src-tauri/tauri.conf.json`，将 `YOUR_PUBLIC_KEY_HERE` 替换为 `public.key` 文件内容。

3. **配置更新源**
   打开 `src-tauri/tauri.conf.json`，将 `YOUR_UPDATE_ENDPOINT_HERE` 替换为实际更新服务器地址。

   支持的更新源：
   - **GitHub Release**：使用 `https://github.com/{owner}/{repo}/releases/latest/download/latest.json`
   - **自建更新服务器**：使用您自己的更新服务器地址

### 更新流程

1. 应用启动时自动检查更新（3秒后后台检查）
2. 发现新版本时显示更新弹窗
3. 用户可选择：
   - **立即更新**：下载并安装更新
   - **稍后提醒**：关闭弹窗，稍后再提示
4. 下载完成后提示用户重启应用

### 手动检查更新

在「设置」页面点击「检查更新」按钮可手动触发更新检查。

## 版本

当前版本：0.1.0 (MVP)

## 许可证

MIT License
