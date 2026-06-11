# Handoff: FrostNote 项目上下文压缩

## Session Metadata
- Created: 2026-06-11 10:20:50
- Project: Z:\label
- Branch: main
- Session duration: 多轮连续开发与修复会话

### Recent Commits (for context)
- 215e909 Remove brand icon white halo
- f74f23a Fix overdue todo rollover status handling
- 884a2e0 Refine compact view and FrostNote branding
- ecf7e99 Update development log after GitHub publish
- 2cefe42 Merge remote initial README

## Handoff Chain

- **Continues from**: None (fresh start)
- **Supersedes**: None

This is the first handoff for this task.

## Current State Summary

FrostNote 是位于 `Z:\label` 的 Windows 桌面便签应用，技术栈为 Tauri 2 + React + TypeScript + Vite + SQLite。项目已完成基础功能、桌面快捷方式、托盘、全局快捷键、紧凑小窗口、README、GitHub 推送，以及多轮 UI 和 bug 修复。当前本地 `main` 与 `origin/main` 同步，最后提交为 `215e909 Remove brand icon white halo`。当前唯一未提交内容是本 handoff 文件本身。

## Codebase Understanding

## Architecture Overview

前端入口在 `src/main.tsx`，负责记录列表、日期切换、输入表单、紧凑模式 UI、搜索筛选和窗口按钮事件。样式集中在 `src/styles.css`，当前视觉方向是白色/天蓝色毛玻璃。后端在 `src-tauri/src/lib.rs`，负责 SQLite 初始化、记录 CRUD、To do 状态切换、逾期滚动、托盘、全局快捷键和窗口控制命令。Tauri 配置在 `src-tauri/tauri.conf.json`，应用名称为 FrostNote，中文名在 UI/文档里为“霜笺”。

## Critical Files

| File | Purpose | Relevance |
|------|---------|-----------|
| `src/main.tsx` | React 主界面和交互逻辑 | 日期、To do、紧凑模式、窗口按钮、跨日刷新都在这里 |
| `src/styles.css` | 全局 UI 样式 | 毛玻璃风格、记录列表滚动、品牌图标白边修复 |
| `src-tauri/src/lib.rs` | Tauri 后端命令和 SQLite 数据层 | 记录持久化、逾期滚动、托盘、快捷键、窗口控制 |
| `src-tauri/tauri.conf.json` | Tauri 构建和窗口配置 | 产品名、窗口尺寸、图标、打包目标 |
| `src/assets/frostnote-icon.png` | 前端品牌图标 | 左上角和 README 图标来源 |
| `src-tauri/icons/icon.ico` | Windows 应用图标 | exe、MSI、NSIS 使用 |
| `frostnote-shortcut-icon.ico` | 桌面快捷方式图标 | `FrostNote.lnk` 指向该文件 |
| `docs/` | 项目标准文档 | 需求、技术、设计、数据模型和开发步骤 |
| `dev-logs/` | 每日开发日志 | 每次开发结束必须更新 |
| `AGENTS.md` | 项目级规则 | 开发前读文档、结束更新日志、中文 UTF-8 注意事项 |

### Key Patterns Discovered

- 用 `npm.cmd` 而不是 `npm` 或 `npm.ps1`，因为 PowerShell 执行策略会阻止 ps1。
- Tauri/Rust 工具链安装在 `Z:\dev-tools`，当前 shell 往往需要临时补环境变量才能运行 `cargo` 或 `tauri build`。
- 应用关闭按钮行为是隐藏到托盘，不是退出；打包前经常需要 `Stop-Process -Name frostnote -Force`，否则 `frostnote.exe` 被占用导致构建失败。
- SQLite 数据库位于 `%APPDATA%\com.frostnote.desktop\frostnote.db`。
- To do 的有效状态必须是 `pending` 或 `done`；普通记录状态为 `NULL`。
- 逾期滚动根据 `rolledOverFromDate` 在前端标红。
- 桌面快捷方式位于当前用户桌面，目标为 `Z:\label\src-tauri\target\release\frostnote.exe`，图标为 `Z:\label\frostnote-shortcut-icon.ico,0`。

## Work Completed

### Tasks Finished

- [x] 建立项目骨架、文档标准和开发日志机制。
- [x] 实现 Tauri + React + TypeScript + Vite 桌面应用。
- [x] 引入 SQLite 本地持久化并迁移旧 localStorage 数据。
- [x] 实现日期归类、普通记录、To do、完成状态、删除线、搜索筛选。
- [x] 实现逾期未完成 To do 自动滚动到今天并红色显示。
- [x] 实现托盘、关闭到托盘、全局快捷键 `Ctrl+Shift+F`、无终端启动。
- [x] 创建桌面快捷方式 `FrostNote.lnk`。
- [x] 将应用名改为 FrostNote，中文名改为“霜笺”。
- [x] 按用户提供样式处理图标，并多次修复白边/白底问题。
- [x] 紧凑小窗口删除编辑/输入/搜索区域，仅保留记录列表和 To do 对勾。
- [x] 修复历史 `status = NULL` 的 To do 不会滚动到今天的问题。
- [x] 修复应用跨午夜不刷新“今天”的问题。
- [x] 编写 README 并推送 GitHub 仓库 `Kyriech7/FrostNote`。

## Files Modified

| File | Changes | Rationale |
|------|---------|-----------|
| `src/main.tsx` | 紧凑模式、中文名、跨日刷新、前端记录展示 | 满足 UI 和 To do 逻辑需求 |
| `src/styles.css` | 毛玻璃 UI、滚动区域、品牌图标容器无白底 | 消除白边并保持整体风格 |
| `src-tauri/src/lib.rs` | SQLite、To do 状态修复、逾期滚动、托盘快捷键、窗口控制 | 核心桌面应用能力 |
| `docs/*.md` | 更新需求、设计规范、数据模型 | 保持项目标准同步 |
| `dev-logs/*.md` | 记录每日工作、验证结果和风险 | 符合项目工作流 |
| `README.md` | 项目介绍、运行、构建、使用说明 | GitHub 展示和后续接手 |
| `*.png` / `*.ico` | FrostNote 图标资源 | 应用、前端、快捷方式图标一致 |

## Decisions Made

| Decision | Options Considered | Rationale |
|----------|-------------------|-----------|
| 使用 `AGENTS.md` | `AGENT.md` 或 `AGENTS.md` | 项目约定使用复数，便于 Agent 自动识别 |
| 使用 SQLite | localStorage 或 SQLite | 桌面应用需要重启持久化和可迁移数据层 |
| 关闭按钮隐藏到托盘 | 直接退出或隐藏 | 更符合便签常驻桌面体验 |
| To do 取消完成回到 `pending` | 回到 `NULL` 或 `pending` | `NULL` 会破坏逾期滚动，To do 状态应始终明确 |
| 品牌图标容器不叠加白色背景 | CSS 底色或透明 PNG | 用户明确不想看到图标周围白边 |
| 下载/工具链放 Z 盘 | C 盘默认路径或 Z 盘 | 用户偏好下载和工具链优先放 Z 盘 |

## Pending Work

## Immediate Next Steps

1. 若继续开发，先运行 `git status --short --branch`，确认除 handoff 外是否有未提交文件。
2. 根据用户下一步反馈继续修 UI 或功能；若涉及图标，注意不要重新引入白色容器底。
3. 每次结束更新当天开发日志，例如 `dev-logs/2026-06-11.md`，并按需提交推送。

### Blockers/Open Questions

- [ ] 当前 handoff 文件尚未提交到 Git；用户如果希望保存到仓库，需要单独提交。
- [ ] `downloads/2026-06-10-toolchain/` 只是安装器备份，用户可能稍后会要求清理。
- [ ] 真实跨午夜刷新只通过代码路径和构建验证，尚未等待真实午夜场景。

### Deferred Items

- 可进一步优化紧凑小窗口的自动化 UI 测试；此前坐标点击容易受前景窗口和系统控件影响。
- 可把 Rust 工具链环境变量写入更稳定的开发脚本，避免每次手动补 PATH。
- 可为数据库迁移和日期滚动增加更多集成测试。

## Context for Resuming Agent

## Important Context

用户当前主要关注 FrostNote 的视觉细节和日常使用 bug，反馈通常配截图。不要只解释，要直接修改并验证。最近一次用户问 `downloads/` 是什么，已确认里面只有 `rustup-init.exe` 和 `vs_BuildTools.exe` 两个开发工具链安装包备份，删除通常不影响运行或构建，因为工具链已安装在 `Z:\dev-tools`。项目已经推送到 GitHub，远程为 `https://github.com/Kyriech7/FrostNote.git`。当前运行/构建常用 release exe 为 `Z:\label\src-tauri\target\release\frostnote.exe`。

## Assumptions Made

- 用户希望继续小步稳定推进，每次修改后验证并记录日志。
- 用户使用 Windows 环境，PowerShell 会显示 `profile.ps1` 执行策略警告；通常可忽略。
- 中文文档优先 UTF-8，避免 Windows 管道编码问题。
- 不要删除 `downloads/`，除非用户明确要求清理。

## Potential Gotchas

- PowerShell 输出中文有时乱码，但文件本身通常是 UTF-8；不要因为终端乱码误改文档。
- 构建前如果 FrostNote 在托盘运行，`tauri build` 可能因为拒绝访问无法覆盖 exe。
- `cargo` 默认可能不可见，需设置：
  - `RUSTUP_HOME=Z:\dev-tools\rustup`
  - `CARGO_HOME=Z:\dev-tools\cargo`
  - `RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc`
  - PATH 前置 `Z:\dev-tools\rustup\toolchains\stable-x86_64-pc-windows-msvc\bin;Z:\dev-tools\cargo\bin`
- UI 截图验证 Tauri 窗口时，坐标点击顶部按钮不稳定；优先用真实截图检查视觉，不要声称未验证的交互已通过。
- 图标白边根因包含 CSS 容器底色和资源边缘像素两部分，后续改图标要同时检查这两层。

## Environment State

### Tools/Services Used

- Node/npm via `npm.cmd`
- Tauri CLI via `npm.cmd run tauri -- ...`
- Rust/Cargo installed under `Z:\dev-tools`
- Visual Studio Build Tools installed under `Z:\dev-tools\VSBuildTools`
- SQLite via `rusqlite` bundled feature
- Git remote `origin` points to GitHub repo `Kyriech7/FrostNote`

### Active Processes

- No dev server intentionally left running.
- No FrostNote process intentionally left running after the last verification.

### Environment Variables

- Relevant names only: `RUSTUP_HOME`, `CARGO_HOME`, `RUSTUP_TOOLCHAIN`, `PATH`, `APPDATA`.
- No secrets are needed for local build.

## Related Resources

- [AGENTS.md](../../AGENTS.md)
- [README.md](../../README.md)
- [docs/requirements.md](../../docs/requirements.md)
- [docs/technical.md](../../docs/technical.md)
- [docs/design-guidelines.md](../../docs/design-guidelines.md)
- [docs/data-model.md](../../docs/data-model.md)
- [docs/development-steps.md](../../docs/development-steps.md)
- [dev-logs/2026-06-10.md](../../dev-logs/2026-06-10.md)
- [dev-logs/2026-06-11.md](../../dev-logs/2026-06-11.md)

---

**Security Reminder**: Validated with `validate_handoff.py`; no secrets intentionally included.
