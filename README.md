# Screen Manager

一款简洁高效的桌面应用使用时间追踪工具，帮助用户了解自己在各个应用上的时间分布。

## 功能特性

### 核心功能
- **时间追踪**：自动记录前台窗口应用的使用时长
- **多维度统计**：支持今日、本周、本月及自定义日期范围统计
- **分类统计**：支持按应用分类（如：工作、娱乐、开发等）进行时间统计
- **开机启动**：可配置开机自动启动，最小化到系统托盘

### 界面展示
- **Dashboard**：实时显示今日使用概况，包括总时长、应用排行、分类占比
- **日报/周报/月报**：详细的时间使用报告，支持环形图和排行榜
- **分类管理**：灵活的应用分类配置

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

## 项目结构

```
screen-manager/
├── src/                      # 前端源代码
│   ├── pages/               # 页面组件
│   │   ├── Dashboard.tsx    # 主页
│   │   ├── Daily.tsx        # 日报
│   │   ├── Weekly.tsx        # 周报
│   │   └── Monthly.tsx       # 月报
│   ├── components/           # 公共组件
│   ├── App.tsx               # 根组件
│   └── main.tsx              # 入口文件
│
└── src-tauri/               # 后端源代码 (Rust)
    ├── src/
    │   ├── main.rs          # 程序入口
    │   ├── database.rs      # 数据库操作
    │   ├── window_monitor.rs # 窗口监控
    │   ├── tray.rs          # 系统托盘
    │   ├── scheduler.rs     # 定时任务
    │   └── autostart.rs     # 开机启动
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

应用数据存储在用户本地 SQLite 数据库中：
- **数据库路径**：`%APPDATA%\screen-manager\screen_manager.db`
- **表结构**：
  - `usage_records`：应用使用记录
  - `app_categories`：应用分类映射
  - `categories`：分类定义

## 版本

当前版本：0.1.0 (MVP)

## 许可证

私有项目
