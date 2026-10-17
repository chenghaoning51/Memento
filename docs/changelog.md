# 变更记录

## 2026-10-17

- 初始化 Tauri 2 + React + TypeScript 项目。
- 将应用标识更新为 `com.memento.app`。
- 建立 Memento 工作台外壳和导航占位页面。
- 建立产品、架构、开发、ADR、路线图和文档模板系统。
- 明确当前阶段尚未接入 SQLite、真实编辑器、附件、自动保存和备份。
- 完成项目文档与代码结构基线核对，新增 MVP 开发计划、迭代流程和验收清单。
- 将后续开发划分为 M0–M6，明确 P0/P1/P2 优先级、阶段依赖、容量假设、任务编号和发布门槛；同步路线图与开发入口。
- 标注数据模型仍是目标设计，记录前端构建与 Rust 检查通过；业务功能、测试工具与安装发布验收仍待实施。

## 2026-10-17 M0 开发准备

- 新增[日记场景细化](product/journal-entry.md)：同日多篇、空日记不自动删除、四态保存状态（已保存/编辑中/保存中/保存失败）、800 ms 停止输入自动保存与最多 5 秒强制保存、切页与关闭前 flush、区分编辑撤销与保存失败恢复与回收站恢复。
- 新增[未来信件场景](product/letters.md)：开启时刻按创建时区换算 UTC、锁定/到期/已打开三状态在启动与进入页面时重算、到期后只能追加回应。
- 新增[导出与备份场景](product/backup-and-export.md)：Markdown 导出规则、完整空间备份清单、恢复预检与失败不半新半旧。
- 新增 [ADR-0002 SQLite 存储](decisions/0002-sqlite-storage.md)：Rust 侧 rusqlite + bundled，迁移使用 `PRAGMA user_version`。
- 新增 [ADR-0003 编辑器与正文格式](decisions/0003-editor-and-body-format.md)：M1 使用原生 textarea 与带版本的 `content_json`，M2 再评估 Tiptap。
- 建立最小质量流程与版本基线。

## 2026-10-17 M1 第一条可靠保存链路

- Rust 侧新增数据目录初始化：启动时在应用数据目录创建 `Memento Library/`，打开 `memento.db`，启用 WAL 与外键，按 `PRAGMA user_version` 顺序执行迁移；未知高版本直接启动失败，不写入。
- 新增五个 Tauri 命令：`library_info`、`list_entries`、`get_entry`、`create_entry`、`save_entry`。
- “今天”页接入真实数据：按本地日历日期列出当天日记、新建日记、切换日记、编辑正文。
- 自动保存：停止输入 800 ms 保存，连续输入最多 5 秒强制保存；同一时刻只有一个在途请求，排队写入最新内容；失败保留输入并显示“重试保存”。
- 切页与关闭窗口前 flush：保存失败时阻止切换并保留当前输入。
- 新增“设置”页，显示数据目录、数据库文件与模式版本。
- 新增窗口权限 `core:window:allow-destroy`，用于关闭请求未被阻止时真正关闭窗口。
