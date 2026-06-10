# 技术方案

## 技术栈

- 桌面容器：Tauri
- 前端框架：React
- 开发语言：TypeScript
- 构建工具：Vite
- 本地数据：优先使用 SQLite
- 目标系统：Windows

## 架构原则

- 前端负责界面、交互、搜索筛选和用户输入状态。
- Tauri 后端负责本地数据读写、托盘、全局快捷键和窗口行为。
- 数据只保存在本机，不依赖账号或网络。
- 每个开发阶段都应保持应用可启动、可验证。

## 模块划分

### 前端模块

- 日期选择和日期侧栏。
- 记录列表。
- 快速输入框。
- 类型筛选和搜索。
- to do 完成按钮与状态显示。
- 白色和天蓝色毛玻璃主题样式。

### 后端模块

- 本地数据库初始化。
- 记录 CRUD。
- 启动时执行逾期 to do 滚动。
- 系统托盘。
- 全局快捷键。
- 关闭窗口时最小化到托盘。

## 本地存储

第一版使用本地 SQLite。数据文件放在 Tauri 推荐的应用数据目录中，避免直接写入项目目录。

当前可点击原型阶段先使用 WebView `localStorage` 保存记录，键名为 `label-notes.records.v1`。这让应用在 SQLite 接入前也能完成新增、编辑、删除、完成状态、搜索筛选和基础持久化；Phase 2 会把该临时存储迁移到 Tauri 后端的 SQLite 数据层。

## 编码注意事项

- 所有用户输入的中文内容必须能正确保存、读取、搜索。
- 文档和脚本生成优先使用 UTF-8。
- Windows 路径处理避免假设类 Unix 路径格式。
- 当前 PowerShell 执行策略可能阻止 `npm.ps1`，项目命令优先使用 `npm.cmd`。
- 功能实现优先简单可靠，避免过早引入复杂状态管理。

## Phase 1 环境记录

- Node.js 可用。
- npm 通过 `npm.cmd` 可用。
- WebView2 已安装。
- Rust、Cargo、rustup 已安装到 `Z:\dev-tools`：
  - `RUSTUP_HOME=Z:\dev-tools\rustup`
  - `CARGO_HOME=Z:\dev-tools\cargo`
  - `Z:\dev-tools\cargo\bin` 已写入用户 Path。
- Visual Studio Build Tools 已安装到 `Z:\dev-tools\VSBuildTools`，包含 MSVC 和 Windows SDK 相关组件。
- 下载的安装器保存在 `Z:\label\downloads\2026-06-10-toolchain`。
- `npm.cmd run tauri -- info` 环境检查已通过。
- `npm.cmd run tauri -- build` 已能生成 Windows 安装包。
- 桌面快捷方式指向 `Z:\label\src-tauri\target\release\frostnote.exe`。
- 如果当前终端仍无法直接识别 `cargo`，重新打开终端，或在命令前临时设置：
  - `$env:RUSTUP_HOME='Z:\dev-tools\rustup'`
  - `$env:CARGO_HOME='Z:\dev-tools\cargo'`
  - `$env:Path='Z:\dev-tools\cargo\bin;' + $env:Path`
