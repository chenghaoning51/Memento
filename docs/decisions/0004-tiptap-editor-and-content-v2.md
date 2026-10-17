# ADR-0004：用 Tiptap 承载正文编辑，正文格式升到 v2

- 状态：Accepted
- 日期：2026-10-17

## 背景

M1 的正文是原生 `<textarea>`，`content_json` 里每个段落只有一个 `text` 字段。M2 需要标题、引用、分隔线、链接和撤销，继续手写 `contenteditable` 会自己实现一套文档模型和撤销栈。

正文同时要满足三个既有约束：完全离线、结构化正文与纯文本副本一起保存（`content_text` 供摘要与后续搜索）、旧数据必须能直接打开。

## 决定

- 引入 Tiptap 3.32（`@tiptap/core`、`@tiptap/react`、`@tiptap/pm`、`@tiptap/starter-kit`、`@tiptap/extensions`、`@tiptap/extension-link`），版本精确锁定，不使用 `^`。
- 正文格式升到 **version 2**：仍是 ProseMirror 文档结构，但允许 `heading`、`blockquote`、`horizontalRule`、`hardBreak` 与 `link` mark。文字一律放在 `{ "type": "text", "text": ... }` 行内节点里。
- 读取时接受 `version <= 2`。version 0/1 的段落 `text` 字段在读取时升级为行内节点；高于 2 抛错，拒绝打开和编辑，避免降级写入丢掉新格式。
- 保存时始终写入 `version: 2`，因此被编辑过的旧日记会在下一次保存时完成升级。
- `content_text` 由 `editor.getText({ blockSeparator: "\n" })` 生成：块之间一个换行，硬换行也是换行。摘要与搜索文本因此与结构化正文始终同源。

## 原因

- Tiptap 直接给出 ProseMirror 文档模型、撤销栈、输入法与粘贴处理，这些在 M1 的 textarea 里是白拿的，自己实现会重造一遍并持续踩坑。
- 文档是 JSON，天然适合存进现有的 `content_json` 列，不需要改表结构或迁移脚本。
- 升级在读取侧完成，数据库里可以同时存在 v1 与 v2 正文，用户不会因为升级而丢失旧日记。
- 版本号放在正文 JSON 内部，和 SQLite 的 `user_version` 各管一层：前者管正文结构，后者管表结构。

## 备选方案

- **继续用 textarea + Markdown**：写起来快，但实时预览、链接点击和撤销粒度都要另做，且正文与纯文本会分叉成两份真相。
- **自研 contenteditable**：可控，但要自己处理光标、输入法组合、粘贴清洗和撤销栈，成本远高于 M2 的收益。
- **换成 Lexical 或 Slate**：能力相当，但 Tiptap 的扩展集合（StarterKit 已含引用、分隔线、链接、撤销）与现有 React 结构贴合，接入代码最少。
- **把版本号存到表里**：会让「一条日记的正文版本」变成跨行信息，读写都要多一次查询，且不好表达「部分旧日记尚未升级」。

## 影响

- `src/lib/entryContent.ts` 负责版本判断与 v1→v2 升级；`src/features/today/EntryEditor.tsx` 承载编辑器与工具栏；`TodayPage` 只持有解析后的 `JSONContent`。
- `src-tauri/src/commands.rs` 的 `EMPTY_CONTENT_JSON` 改为 version 2 的空段落，新建日记直接落在新格式上。
- 前端包体明显变大（构建产物约 651 kB），M2 不做拆分；如果后续继续增长，再考虑按需加载。
- 正文格式一旦再变，必须继续升版本号并在 `readContent` 里补升级分支，不能就地改写 v2 的含义。
