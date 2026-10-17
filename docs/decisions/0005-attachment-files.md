# ADR-0005：图片存为附件文件，正文只记附件 ID

- 状态：Accepted
- 日期：2026-10-17

## 背景

M2-02 要让日记能插入图片。约束有三条：完全离线；原始图片不能以 Base64 长期存在数据库里；移动或删除用户原始文件后，日记里的图片仍然可读。

SQLite 里已经有 `entries.cover_attachment_id` 和 `attachments` 的目标设计，但"图片存在哪、怎么引用、怎么读回来"没有定。可选项包括 Base64 内嵌、绝对路径引用和复制到应用目录。

## 决定

- 原始图片**复制**进数据目录：`attachments/<entry_id>/<attachment_id>.<ext>`；缩略图在 `thumbnails/<attachment_id>.jpg`，长边 320、JPEG 质量 75。路径一律相对 `Memento Library/` 保存，不存绝对路径。
- 只接受 PNG 与 JPEG，用 `image` crate 的 `guess_format` 按内容判断，不看扩展名；其它格式报"只支持 PNG 和 JPEG 图片。"，不写任何文件。
- `attachments` 表存元数据：`id`、`entry_id`、`relative_path`、`mime_type`、`file_size`、`sha256`、`width`、`height`、`created_at_utc`。
- 正文 JSON 的 image 节点只记 `attachmentId` 与展示用的 `width`/`height`/`alt`，不记路径、不记 Base64。列表首图读 `entries.cover_attachment_id`，取正文里第一张图的 ID。
- 读图走自定义协议：`convertFileSrc(id, "attachment")` 生成 URL，Rust 侧 `register_uri_scheme_protocol("attachment", ...)` 按 ID 查表拿相对路径再读文件。前端不知道也不需要知道磁盘布局。
- 导入顺序：先写原图，再写缩略图，再插数据库行。任一步失败就删掉已写的文件并返回错误，正文不变。删除附件时先删数据库行（拿到相对路径），再删两个文件。
- **删除图片节点不自动清理附件**。软删除日记同样不动附件。永久删除与自动清理的产品规则未确定（见 [journal-entry.md](../product/journal-entry.md)），M2 不自行加入。

## 原因

- 复制进应用目录，才能做到"原文件删除或移动后仍可阅读"（A07 的通过标准），也才谈得上 M5 的完整备份与校验。
- 正文只记 ID，路径可以随备份/恢复整体迁移而不必改写正文；相对路径进表则是为了恢复时能整体换根目录。
- 自定义协议而不是把图片读成 Base64 塞回前端：避免正文和内存里出现图片副本，也让 `<img src>` 直接可用、浏览器缓存生效。
- 先写文件再插行，失败回滚文件，保证不会出现"数据库说有一张图，磁盘上没有"。
- 不自动清理孤儿附件是有意的：清理需要判断回收站、历史正文和未来备份的一致性，规则没定就自动删文件，等于把用户的图片悄悄删掉。

## 备选方案

- **Base64 存在 `content_json` 里**：正文体积爆炸，每次保存都要重写整张图，备份和搜索都变慢。
- **只存用户原文件的绝对路径**：用户移动或删除源文件后图片就没了，备份也无法还原到别的机器。
- **存绝对路径 + 复制**：仍会把某台机器的目录结构写死进数据库，恢复换目录就断链。
- **用 `asset:` 协议直接读任意路径**：等于把整个文件系统的读权限暴露给前端，破坏"UI 不直接访问文件系统"的边界。
- **引入 base64 或 `blob:` 方案传输**：离线缓存里没有合适的依赖，且会给每次导入多一份编码开销；`Vec<u8>` 数字数组直接过 IPC 已经够用。

## 影响

- `src-tauri/src/attachments/mod.rs` 负责格式识别、缩略图、SHA-256 和路径规则；`attachments/repository.rs` 负责元数据读写；`commands.rs` 负责导入/删除命令与自定义协议响应。
- 迁移 v2 新增 `attachments` 表与 `attachments_by_entry`、`attachments_by_sha256` 索引。
- 新增 Rust 依赖 `image` 0.25，关闭默认特性，只开 `png` 与 `jpeg`，避免引入用不到的编解码器。
- 前端新增 `src/lib/attachmentSource.ts`；`EntryEditor` 负责选图、导入与插入节点，`TodayPage` 负责首图与保存。
- 孤儿附件会留在磁盘上。M5 的备份必须连同 `attachments/` 与 `thumbnails/` 一起打包，否则恢复后图片会缺失；是否清理孤儿文件留到永久删除规则确定时再定。
