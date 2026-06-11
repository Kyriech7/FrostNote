# Supabase 设置步骤

本文档用于配置 FrostNote Phase 9 云同步第一版。

## 1. 创建项目

- 登录 Supabase 控制台。
- 创建一个新项目。
- 记录项目的 API URL 和 publishable key。
- 不要使用 service role key 作为桌面端环境变量。

## 2. 配置 Auth

- 打开 Authentication 设置。
- 第一版关闭邮箱验证，让邮箱密码注册后可直接登录。
- 登录方式使用 Email + Password。

## 3. 初始化数据库

- 打开 SQL Editor。
- 执行 `docs/supabase-schema.sql` 中的 SQL。
- 确认 `public.records` 表已创建。
- 确认 `public.profiles` 表已创建，用于存储用户自定义 UID。
- 确认 `public.get_my_custom_uid()` 函数和 `public.check_custom_uid_available()` 函数已创建。
- 确认 `on_auth_user_created` 触发器已在 `auth.users` 上启用。
- 确认 RLS 已启用，并存在 select、insert、update、delete 四条 own-record policies。

## 4. 配置本地环境变量

在项目根目录创建 `.env.local`，内容参考：

```env
VITE_SUPABASE_URL=https://your-project-ref.supabase.co
VITE_SUPABASE_PUBLISHABLE_KEY=your-supabase-publishable-key
```

`.env.local` 已被 `.gitignore` 忽略，不要提交真实值。

## 5. 验证流程

- 运行 `npm.cmd run build`，确认环境变量不会破坏前端构建。
- 运行 `npm.cmd run tauri -- build`，确认桌面 release 可构建。
- 启动 FrostNote，注册一个测试账号。
- 新增一条 FreeNote 和一条 To do。
- 点击手动同步。
- 在 Supabase Table Editor 中确认 `records` 表出现对应记录，且 `user_id` 为当前账号。
- 删除其中一条记录后再次同步，确认云端该行写入 `deleted_at`，而不是被物理删除。
- 清空或备份本地数据库后，使用同一账号登录，确认云端记录可拉回。

## 6. 当前同步约束

- FrostNote 仍是本地优先，断网时本地记录能力不受影响。
- 只有本地 `sync_status='dirty'` 的记录会上传。
- 同一 `id` 的记录冲突时，保留 `updatedAt` 更新的一方。
- 删除记录通过 `deletedAt` tombstone 同步，UI 默认隐藏已删除记录。
