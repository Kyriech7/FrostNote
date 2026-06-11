# 数据模型

## Record

所有普通记录和 to do 事项都使用同一类记录模型，通过 `type` 区分。

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `id` | string | 唯一标识 |
| `type` | `note` 或 `todo` | 记录类型 |
| `content` | string | 单段文本内容 |
| `date` | string | 归属日期，格式为 `YYYY-MM-DD` |
| `status` | `pending` 或 `done` 或 null | to do 状态；普通记录为空；to do 不应为空 |
| `createdAt` | string | 创建时间，ISO 字符串 |
| `updatedAt` | string | 更新时间，ISO 字符串 |
| `completedAt` | string 或 null | 完成时间，仅 to do 使用 |
| `rolledOverFromDate` | string 或 null | 逾期滚动来源日期 |
| `deletedAt` | string 或 null | 软删除时间；为空表示可见 |
| `syncStatus` | `dirty` 或 `synced` | 仅本地 SQLite 使用；表示是否需要重新上传 |

## 记录类型规则

### 普通记录

- `type` 为 `note`。
- `status` 为 null。
- 不参与完成状态和逾期滚动。
- 删除时不直接物理删除，写入 `deletedAt` 作为同步 tombstone。

### to do 事项

- `type` 为 `todo`。
- 新建时 `status` 为 `pending`。
- 点击对勾后 `status` 变为 `done`。
- 已完成事项再次点击对勾时，`status` 必须回到 `pending`，不能写入 null。
- 历史数据中若存在 `type = todo` 且 `status` 为空，会在数据库初始化或旧数据导入后自动迁正为 `pending` 或 `done`。
- 完成时写入 `completedAt`。
- 完成事项保留在列表中，前端显示删除线。
- 删除时不直接物理删除，写入 `deletedAt` 作为同步 tombstone。

## 日期规则

- 所有记录都有 `date`。
- 新建记录默认 `date` 为今天。
- 用户可以选择过去、今天或未来日期。
- 日期按本地 Windows 日期计算。

## 逾期滚动规则

应用启动或本地日期跨天时检查所有未完成 to do：

- 条件：`type = todo`，`status = pending`，且 `date` 早于今天。
- 操作：将 `date` 更新为今天。
- 同时将原日期写入 `rolledOverFromDate`。
- 前端根据 `rolledOverFromDate` 不为空且状态未完成，将该事项显示为红色。
- 已完成 to do 不参与滚动。
- 普通记录不参与滚动。
- 已软删除记录不参与滚动。

## 同步规则

- 未登录时，所有数据只保存在本地 SQLite。
- 登录后，本地 `syncStatus = dirty` 的记录会同步到 Supabase `records` 表。
- 云端用户标识：注册时用户选择自定义 UID（3-30 位字母数字），存储在 `public.profiles` 表中，通过 `auth.users` 的 `user_metadata` 传入。`records.user_id` 引用该自定义 UID。
- 本地新增、编辑、完成状态切换、逾期滚动和删除都会把 `syncStatus` 标记为 `dirty`。
- 同步成功后，已上传的本地记录标记为 `synced`。
- 同一 `id` 的本地和云端记录发生冲突时，保留 `updatedAt` 更新的一方。
- UI 默认只展示 `deletedAt` 为空的记录；同步接口可读取带 `deletedAt` 的 tombstone。

## 搜索与筛选

- 搜索范围为 `content`。
- 筛选类型包括：
  - 全部
  - 普通记录
  - to do
  - 未完成
  - 已完成
