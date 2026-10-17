# 数据模型

## 当前状态

M1 已创建数据库与迁移系统；M2-02 的迁移 v2 加入 `attachments` 表，M2-03 的迁移 v3 加入 `tags` 与 `entry_tags`。当前实际落地的四张表：

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

```sql
CREATE TABLE attachments (
  id             TEXT PRIMARY KEY NOT NULL,
  entry_id       TEXT NOT NULL REFERENCES entries (id),
  relative_path  TEXT NOT NULL,
  mime_type      TEXT NOT NULL,
  file_size      INTEGER NOT NULL,
  sha256         TEXT NOT NULL,
  width          INTEGER NOT NULL,
  height         INTEGER NOT NULL,
  created_at_utc TEXT NOT NULL
);
CREATE INDEX attachments_by_entry  ON attachments (entry_id);
CREATE INDEX attachments_by_sha256 ON attachments (sha256);
```

```sql
CREATE TABLE tags (
  id              TEXT PRIMARY KEY NOT NULL,
  name            TEXT NOT NULL,
  normalized_name TEXT NOT NULL UNIQUE
);
CREATE TABLE entry_tags (
  entry_id TEXT NOT NULL REFERENCES entries (id),
  tag_id   TEXT NOT NULL REFERENCES tags (id),
  PRIMARY KEY (entry_id, tag_id)
);
CREATE INDEX entry_tags_by_tag ON entry_tags (tag_id);
```

迁移版本记录在 `PRAGMA user_version`，不再使用 `schema_meta` 表（见 [ADR-0002](../decisions/0002-sqlite-storage.md)）。`letters`、`settings` 仍为目标设计，随 M4 的迁移增量加入；备份容器格式在 M5 前确定。具体契约改变或选型前先写 ADR。

## 设计目标

数据模型要支持正文结构、附件、时间回看、标签和未来信件，同时允许未来加入收藏卡片而不破坏日记数据。

## 核心表

`entries`、`attachments`、`tags`、`entry_tags` 已实现，字段见上文建表语句。以下为目标设计：

```text
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
- 日记删除使用软删除；永久删除需要单独的清理动作。M1 只写入 `deleted_at_utc`，正常列表按 `deleted_at_utc IS NULL` 过滤，回收站列表按 `deleted_at_utc DESC` 取被删项；恢复即把该列写回 NULL，正文、附件行与标签关系都不动。
- 标签名规范化后存 `name`（去首尾空白、内部连续空白折叠为一个空格），再小写存 `normalized_name` 并加唯一约束；输入与比较都走同一套规则，因此同一标签在不同日记里复用同一行。`entry_tags` 用复合主键去重，删除日记不删关系。
- 附件保存 SHA-256，用于备份校验和重复文件识别。原图复制进 `attachments/<entry_id>/<attachment_id>.<ext>`，缩略图在 `thumbnails/<attachment_id>.jpg`；表里只存相对路径，不存绝对路径。附件行与文件的一致性由导入顺序保证（先文件后行，失败回滚文件），删除图片节点或软删除日记都不自动清理附件，见 [ADR-0005](../decisions/0005-attachment-files.md)。
- 编辑器正文同时保存结构化 JSON 和纯文本副本，后者用于摘要和搜索。当前格式为 version 2，即 ProseMirror 文档，例如 `{"version":2,"type":"doc","content":[{"type":"heading","attrs":{"level":2},"content":[{"type":"text","text":"标题"}]}]}`；version 1 把文字放在段落的 `text` 字段里，读取时自动升级为行内节点。未知高版本拒绝编辑，见 [ADR-0004](../decisions/0004-tiptap-editor-and-content-v2.md)。
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
