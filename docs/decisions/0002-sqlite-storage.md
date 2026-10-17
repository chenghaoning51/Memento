# ADR-0002：使用 rusqlite 访问本地 SQLite

- 状态：Accepted
- 日期：2026-10-17

## 背景

Memento 需要一个完全离线的本地数据库保存日记、标签、附件元数据和未来信件。数据必须能被可靠迁移、备份和恢复，并且不依赖任何后端服务。

可选方案有三类：Rust 侧直接用 `rusqlite`；使用 Tauri 官方 `tauri-plugin-sql`；使用纯前端 SQLite（sql.js / wa-sqlite）。

## 决定

在 Rust 侧使用 `rusqlite`，开启 `bundled` 特性编译 SQLite，数据库连接由 Tauri 管理的应用状态持有，只通过类型化的 Tauri 命令暴露给前端。

## 原因

- `bundled` 让 SQLite 版本随二进制固定，Windows 用户不需要预装任何运行库，验收环境可控。
- 业务规则（迁移顺序、事务边界、软删除、校验值）留在 Rust，符合“UI 不直接访问数据库”的架构约束。
- `tauri-plugin-sql` 把 SQL 语句暴露给前端，会让组件散落 SQL，难以保证事务和迁移顺序，和现有架构文档冲突。
- 纯前端 SQLite 需要把整个数据库载入内存再写回文件，无法提供可靠的写入成功语义，和 M1 的“持久化成功后才显示已保存”直接冲突。

## 备选方案

- `tauri-plugin-sql`：开发快，但 SQL 泄漏到前端，迁移与事务边界不受控，未采用。
- sql.js / wa-sqlite：无法保证崩溃时的持久化语义，未采用。
- 直接使用 `libsqlite3-sys`：需要手写大量绑定代码，没有收益，未采用。

## 迁移策略

- 使用 SQLite 内建的 `PRAGMA user_version` 作为模式版本，不额外建 `schema_meta` 表；`user_version` 是文件级元数据，随数据库文件一起被备份带走，不需要额外同步。
- 迁移是 `[函数]` 的有序列表，启动时在单个事务内从当前版本顺序执行到目标版本。
- 如果数据库的 `user_version` 高于程序支持的最大版本，启动直接失败并说明原因，不写入、不清空。
- 迁移失败时回滚事务，保留旧版本数据。

## 影响

- 新增依赖：`rusqlite`（bundled）、`uuid`（v4 主键）、`time`（UTC 时间戳格式化与解析）。
- 数据库文件位于应用数据目录下的 `Memento Library/memento.db`，与附件、缩略图、备份、导出同级。
- 所有读写都在 Rust 的应用服务和仓储层完成；前端只看到结构化的命令返回值。
- 备份可以直接使用 SQLite 的在线备份 API 或一致性快照，不需要理解业务表结构。
