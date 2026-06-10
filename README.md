# FrostNote

![FrostNote icon](src/assets/frostnote-icon.png)

FrostNote 是一个面向 Windows 的轻量桌面便签应用，用来快速记录 FreeNote 和 To do 事项。它采用白色与天蓝色毛玻璃风格，支持按日期归类、本地 SQLite 持久化、逾期待办自动滚动、系统托盘和全局快捷键。

## 功能特性

- FreeNote：记录普通文字事项，支持新增、编辑、删除和搜索。
- To do：支持未完成/已完成状态，完成后保留在列表中并显示删除线。
- 日期归类：默认显示今天，可切换过去和未来日期，也可提前规划事项。
- 逾期滚动：未完成且超过归属日期的 To do 会在启动时自动移动到今天，并用红色样式提示。
- 搜索与筛选：支持全文搜索，并可按全部、FreeNote、To do、未完成、已完成筛选。
- 清除已完成：可一键清理当天已完成的 To do。
- 紧凑模式：窗口缩小为右上角小面板，适合临时记录和快速查看今天事项。
- Windows 桌面体验：支持系统托盘、关闭到托盘、自定义窗口按钮和全局快捷键。
- 本地优先：数据保存在本机 SQLite 数据库，不依赖账号、云同步或网络服务。

## 技术栈

| 层级 | 技术 |
| --- | --- |
| 桌面容器 | Tauri 2 |
| 前端 | React 19 + TypeScript |
| 构建 | Vite |
| 本地数据 | SQLite, rusqlite bundled |
| Windows 能力 | Tauri tray icon, global shortcut, custom window controls |

## 运行环境

建议在 Windows 10/11 上开发和运行。

需要准备：

- Node.js
- npm
- Rust stable toolchain
- Microsoft Visual Studio Build Tools, 包含 MSVC 和 Windows SDK
- Microsoft Edge WebView2 Runtime

PowerShell 环境下建议使用 `npm.cmd`，避免执行策略拦截 `npm.ps1`。

## 本地开发

安装依赖：

```powershell
npm install
```

启动前端开发服务：

```powershell
npm.cmd run dev
```

启动 Tauri 桌面开发模式：

```powershell
npm.cmd run tauri -- dev
```

## 构建安装包

先构建前端：

```powershell
npm.cmd run build
```

构建 Windows release 和安装包：

```powershell
npm.cmd run tauri -- build
```

构建完成后，主要产物位于：

```text
src-tauri/target/release/frostnote.exe
src-tauri/target/release/bundle/msi/
src-tauri/target/release/bundle/nsis/
```

## 使用说明

- 使用顶部输入区快速新增记录。
- 在类型切换中选择 `FreeNote` 或 `To do`。
- To do 右侧的对勾按钮用于切换完成状态。
- 已完成 To do 不会自动删除，会保留删除线和完成时间。
- 使用日期导航按钮切换前一天、今天和后一天。
- 点击紧凑模式按钮 `⊡` 可进入小窗口模式，再点击 `⊠` 还原。
- 点击窗口关闭按钮会隐藏到系统托盘，而不是退出应用。
- 托盘右键菜单可显示 FrostNote 或退出应用。
- 全局快捷键 `Ctrl+Shift+F` 可切换窗口显示/隐藏。

## 数据存储

FrostNote 使用 SQLite 保存本地数据。数据库文件位于 Tauri 应用数据目录：

```text
%APPDATA%/com.frostnote.desktop/frostnote.db
```

早期版本的 `localStorage` 数据会在首次启动时自动迁移到 SQLite，迁移成功后清除旧数据键。

## 项目结构

```text
.
├─ src/                    # React 前端
│  ├─ main.tsx             # 主界面、记录交互、Tauri invoke 调用
│  ├─ styles.css           # 白色/天蓝色毛玻璃 UI
│  └─ assets/              # 前端图标资源
├─ src-tauri/              # Tauri/Rust 后端
│  ├─ src/lib.rs           # SQLite、托盘、快捷键、窗口命令
│  ├─ src/main.rs          # Windows GUI 子系统入口
│  ├─ capabilities/        # Tauri 权限配置
│  └─ icons/               # Windows 应用图标
├─ docs/                   # 项目需求、技术方案、设计规范和数据模型
├─ dev-logs/               # 每日开发日志
├─ AGENTS.md               # Agent 工作说明
├─ package.json            # 前端脚本和依赖
└─ README.md
```

## 设计方向

FrostNote 的界面保持安静、轻量和清爽：主背景接近白色，主强调色为天蓝色，面板使用低透明度毛玻璃和克制阴影。逾期事项使用红色提示，已完成事项降低对比度并显示删除线。

## 开发文档

- [产品需求](docs/requirements.md)
- [技术方案](docs/technical.md)
- [设计规范](docs/design-guidelines.md)
- [数据模型](docs/data-model.md)
- [开发步骤](docs/development-steps.md)
- [开发日志](dev-logs/2026-06-10.md)

## 当前状态

FrostNote v0.1.0 已完成核心桌面便签流程，包括 FreeNote、To do、日期归类、SQLite 持久化、逾期滚动、搜索筛选、紧凑模式、系统托盘和全局快捷键。
