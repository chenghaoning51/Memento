# 架构决策记录

架构决策记录（ADR）用于保存影响模块边界、数据格式、依赖和用户承诺的决定。代码可以变化，决定背后的原因仍然应该可追溯。

## 什么时候写 ADR

- 引入或替换核心依赖。
- 改变数据库或备份格式。
- 改变本地数据安全边界。
- 选择会影响未来同步、导入或迁移的实现。
- 发现旧决定不再适用，需要废弃或替代。

## 状态

- `Proposed`：正在讨论。
- `Accepted`：当前采用。
- `Superseded`：被后续决定替代。
- `Rejected`：明确不采用。

## 模板

复制 [`templates/adr.md`](../templates/adr.md)，使用四位数字和短标题命名，例如 `0002-sqlite-storage.md`。

## 当前决定

- [0001：选择 Tauri + React + TypeScript](0001-tauri-react-local-first.md)
- [0002：使用 rusqlite 访问本地 SQLite](0002-sqlite-storage.md)
- [0003：M1 使用结构化纯文本正文，M2 再引入 Tiptap](0003-editor-and-body-format.md)
- [0004：用 Tiptap 承载正文编辑，正文格式升到 v2](0004-tiptap-editor-and-content-v2.md)
