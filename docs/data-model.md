# 数据模型

## Record

所有普通记录和 to do 事项都使用同一类记录模型，通过 `type` 区分。

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `id` | string | 唯一标识 |
| `type` | `note` 或 `todo` | 记录类型 |
| `content` | string | 单段文本内容 |
| `date` | string | 归属日期，格式为 `YYYY-MM-DD` |
| `status` | `pending` 或 `done` 或 null | to do 状态，普通记录为空 |
| `createdAt` | string | 创建时间，ISO 字符串 |
| `updatedAt` | string | 更新时间，ISO 字符串 |
| `completedAt` | string 或 null | 完成时间，仅 to do 使用 |
| `rolledOverFromDate` | string 或 null | 逾期滚动来源日期 |

## 记录类型规则

### 普通记录

- `type` 为 `note`。
- `status` 为 null。
- 不参与完成状态和逾期滚动。

### to do 事项

- `type` 为 `todo`。
- 新建时 `status` 为 `pending`。
- 点击对勾后 `status` 变为 `done`。
- 完成时写入 `completedAt`。
- 完成事项保留在列表中，前端显示删除线。

## 日期规则

- 所有记录都有 `date`。
- 新建记录默认 `date` 为今天。
- 用户可以选择过去、今天或未来日期。
- 日期按本地 Windows 日期计算。

## 逾期滚动规则

应用启动时检查所有未完成 to do：

- 条件：`type = todo`，`status = pending`，且 `date` 早于今天。
- 操作：将 `date` 更新为今天。
- 同时将原日期写入 `rolledOverFromDate`。
- 前端根据 `rolledOverFromDate` 不为空且状态未完成，将该事项显示为红色。
- 已完成 to do 不参与滚动。
- 普通记录不参与滚动。

## 搜索与筛选

- 搜索范围为 `content`。
- 筛选类型包括：
  - 全部
  - 普通记录
  - to do
  - 未完成
  - 已完成

