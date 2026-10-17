# 数据模型

## 当前状态

M1 已创建数据库与迁移系统，实际落地的只有 `entries` 表：

```sql
CREATE TABLE entries (
  id                  TEXT PRIMARY KEY,
  entry_date          TEXT NOT NULL,
  title               TEXT NOT NULL DEFAULT '',
  content_json        TEXT NOT NULL,
  content_text        TEXT NOT NULL DEFAULT '',
  excerpt             TEXT NOT NULL DEFAULT '',
  layout_style        TEXT NOT NULL DEFAULT 'default',
  cover_attachment_id TEXT,
  created_at_utc      TEXT NOT NULL,
  updated_at_utc      TEXT NOT NULL,
  deleted_at_utc      TEXT
);
CREATE INDEX entries_by_date    ON entries (entry_date, created_at_utc);
CREATE INDEX entries_by_deleted ON entries (deleted_at_utc);
```

迁移版本记录在 `PRAGMA user_version`，不再使用 `schema_meta` 表（见 [ADR-0002](../decisions/0002-sqlite-storage.md)）。`attachments`、`tags`、`entry_tags`、`letters`、`settings` 仍为目标设计，随 M2–M4 的迁移增量加入；备份容器格式在 M5 前确定。具体契约改变或选型前先写 ADR。

## 设计目标

数据模型要支持正文结构、附件、时间回看、标签和未来信件，同时允许未来加入收藏卡片而不破坏日记数据。

## 核心表

`entries` 已在 M1 实现，字段见上文建表语句。以下为目标设计：

```text
attachments
- id
- entry_id
- relative_path
- mime_type
- file_size
- sha256
- width
- height
- created_at_utc

tags
- id
- name
- normalized_name

entry_tags
- entry_id
- tag_id

letters
- id
- title
- body_json
- body_text
- unlock_at_utc
- unlock_timezone
- status
- opened_at_utc
- reply_json
- created_at_utc

settings
- key
- value
```

模式版本不使用表，由 `PRAGMA user_version` 保存。

## 约束

- 所有主键使用稳定 UUID，避免导入和恢复时依赖数据库自增值。M1 的 `id` 由 Rust 侧生成 UUID v4。
- 日记删除使用软删除；永久删除需要单独的清理动作。M1 只写入 `deleted_at_utc`，正常列表按 `deleted_at_utc IS NULL` 过滤。
- 附件保存 SHA-256，用于备份校验和重复文件识别。
- 编辑器正文同时保存结构化 JSON 和纯文本副本，后者用于摘要和搜索。M1 的 JSON 是 `{"version":1,"type":"doc","content":[{"type":"paragraph","text":"..."}]}`，未知高版本拒绝编辑。
- `excerpt` 由 Rust 侧从 `content_text` 折叠空白后截取前 80 字符，不信任前端传入。
- 数据库迁移必须带版本号，并在启动时按顺序执行；`user_version` 高于当前支持版本时启动失败并拒绝写入。

## 搜索策略

第一版先实现标题、正文纯文本的规范化子串搜索，加上标签和日期过滤。SQLite 默认 FTS5 分词对中文并不天然可靠，达到较大数据量后再评估中文分词或 n-gram 索引。

## 备份格式

完整备份应包含：

1. 数据库或可迁移的数据快照。
2. `attachments/` 和缩略图。
3. `manifest.json`，记录应用版本、模式版本、文件清单和校验值。
4. 恢复前的当前空间备份。
