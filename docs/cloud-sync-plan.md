# FrostNote 云同步第一版计划

## Summary

为 FrostNote 增加“账号 + 云同步”，但保持本地优先：未登录或断网时仍完整使用本地 SQLite；登录后通过 Supabase 同步记录。第一版不做网页端、开放 API、分享、团队协作或复杂冲突界面。

默认决策：

- 后端：Supabase Auth + Postgres + RLS。
- 登录：邮箱密码，第一版关闭邮箱验证。
- 同步：本地优先，启动、登录、保存后自动同步，并保留手动同步按钮。
- 冲突：`updatedAt` 最新者胜出。
- 会话：Supabase SDK 默认 WebView 本地持久化。
- 安全：应用内只放 Supabase publishable key，不放 service role key。

## Key Changes

- 先更新文档：`docs/requirements.md`、`docs/technical.md`、`docs/data-model.md`、`docs/development-steps.md`，新增云同步阶段和“仍支持本地模式”的约束。
- 新增 Supabase 配置：
  - 创建 Supabase 项目。
  - 关闭 Confirm email。
  - 新增 `.env.example`，包含 `VITE_SUPABASE_URL`、`VITE_SUPABASE_PUBLISHABLE_KEY`。
  - 更新 `.gitignore`，忽略 `.env`、`.env.local`。
- 新增云端 `records` 表：
  - 字段沿用本地模型：`id`、`type`、`content`、`date`、`status`、`created_at`、`updated_at`、`completed_at`、`rolled_over_from_date`。
  - 新增 `user_id uuid`，关联 `auth.users(id)`。
  - 新增 `deleted_at` 支持软删除同步。
  - 启用 RLS，只允许用户读写自己的记录。
- 本地 SQLite 增加：
  - `deleted_at TEXT`
  - `sync_status TEXT NOT NULL DEFAULT 'dirty'`
  - 删除记录改为软删除，UI 默认隐藏 `deleted_at` 不为空的记录。
- 前端增加账号和同步 UI：
  - 未登录：显示“本地模式”和“登录同步”入口。
  - 已登录：显示账号邮箱、同步状态、手动同步、退出登录。
  - 登录/注册表单使用邮箱密码。
- 新增同步模块：
  - `@supabase/supabase-js` 作为唯一云端客户端。
  - 登录后先上传本地 dirty 记录，再拉取云端记录并合并。
  - 合并规则：同一 `id` 取 `updatedAt` 更晚者；删除也是一条带 `deletedAt` 的变更。
  - 网络失败不阻塞本地保存，只显示同步失败状态，下一次自动或手动同步重试。

## Interfaces And Types

- `RecordItem` 新增：
  - `deletedAt: string | null`
  - 本地内部可额外维护 `syncStatus: "dirty" | "synced"`，不必显示给用户。
- 新增前端模块建议：
  - `src/cloud/supabaseClient.ts`：创建 Supabase client。
  - `src/cloud/sync.ts`：封装上传、拉取、合并逻辑。
  - `src/cloud/auth.ts`：封装注册、登录、登出、会话监听。
- 新增或调整 Tauri 命令：
  - `get_all_records` 继续只返回未删除记录给 UI。
  - 新增 `get_sync_records` 返回包含 tombstone 的同步数据。
  - `delete_record` 改为设置 `deleted_at` 和 `sync_status='dirty'`。
  - 新增 `mark_records_synced` 或等价命令，在同步成功后更新本地同步状态。

## Test Plan

- Rust 单元测试：
  - SQLite 迁移后旧数据仍可读取。
  - 删除记录写入 `deleted_at`，不会出现在普通列表。
  - To do 逾期滚动逻辑不受同步字段影响。
- 前端/集成验证：
  - 未登录时新增、编辑、删除、搜索、重启恢复仍正常。
  - 注册、登录、退出后 UI 状态正确。
  - 首次登录会把本地现有记录上传到 Supabase。
  - 第二台或清空本地数据库后登录，可从云端拉回记录。
  - 双端修改同一条记录时，`updatedAt` 较新的版本胜出。
  - 一端删除记录后，另一端同步后该记录消失且不会复活。
  - 断网或 Supabase 不可达时，本地保存成功，状态显示同步失败。
- 命令验证：
  - `cargo test`
  - `npm.cmd run build`
  - `npm.cmd run tauri -- build`
  - 结束后更新当天开发日志。

## Assumptions

- 第一版只服务 FrostNote 桌面端，不做网页端和开放网络 API。
- Supabase 项目尚未创建，实施时需要创建项目并执行 SQL。
- 不提交真实 `.env.local`。
- 邮箱验证先关闭；未来公开发布前可再补深链接或邮件确认流程。
- 参考官方资料：Supabase React 安装与环境变量、邮箱密码 Auth、RLS 文档。
  - https://supabase.com/docs/guides/auth/passwords
  - https://supabase.com/docs/guides/database/postgres/row-level-security
  - https://supabase.com/docs/reference/javascript/installing
