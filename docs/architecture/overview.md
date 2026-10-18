# 系统架构

## 当前状态

M1 打通了“创建 → 编辑 → 自动保存 → 关闭重开”的第一条可靠保存链路，M2 起正文换成结构化编辑器，M3 补上时间回看与检索，M4-01 加入未来信箱，M5 加入 Markdown 导出与完整备份恢复：

- Rust 侧在应用数据目录下创建 `Memento Library/`，用 rusqlite（bundled）打开 `memento.db`，启用 WAL 与外键，并通过 `PRAGMA user_version` 顺序执行迁移（见 [ADR-0002](../decisions/0002-sqlite-storage.md)）。
- 二十八个 Tauri 命令对前端可用：`library_info`、`list_entries`、`list_entry_dates`、`list_timeline`、`search_entries`、`list_tags`、`get_entry`、`create_entry`、`save_entry`、`import_attachment`、`delete_attachment`、`list_deleted_entries`、`delete_entry`、`restore_entry`、`list_entry_tags`、`add_entry_tag`、`remove_entry_tag`、`list_letters`、`get_letter`、`create_letter`、`update_letter`、`delete_letter`、`open_letter`、`save_letter_reply`、`export_entries`、`create_backup`、`inspect_backup`、`restore_backup`；SQL 与业务规则只存在于 Rust 侧。
- 日记页是真实数据流：按传入的本地日历日期列出当天日记，可新建与切换，停止输入 800 ms 自动保存、连续输入最多 5 秒强制保存（见 [ADR-0003](../decisions/0003-editor-and-body-format.md)）。“今天”与“某一天”是同一个页面，只是日期不同。
- 时间回看分两条只读查询：`list_timeline` 按日记日期倒序、同一天内后写的在前分页；`list_entry_dates` 按日期范围汇总每天篇数与当天首图，供月历一次取整屏（含相邻月格子）。排序、分组与统计都在 SQL 里完成，前端只做渲染。
- 正文用 Tiptap 编辑，支持标题、引用、分隔线、链接、加粗、斜体和撤销重做；结构化 JSON 存 `content_json`，纯文本副本存 `content_text` 供摘要与后续搜索使用。正文格式当前为 version 2，旧的 version 1 正文在读取时自动升级（见 [ADR-0004](../decisions/0004-tiptap-editor-and-content-v2.md)）。
- 图片以附件文件保存：导入时把原图复制到 `attachments/<entry_id>/<attachment_id>.<ext>` 并生成缩略图，正文 image 节点只记附件 ID；编辑器通过 `attachment` 自定义协议按 ID 读图，前端不接触磁盘路径（见 [ADR-0005](../decisions/0005-attachment-files.md)）。
- 标签独立存表：编辑器上方的标签栏在确认和移除时立即写库，规范化后按 `normalized_name` 复用同一行；正文里不存标签字符串（见 [ADR-0006](../decisions/0006-tags-and-soft-delete.md)）。
- 删除日记是软删除：页脚的“移到回收站”只写 `deleted_at_utc`，日记从“今天”列表消失；“回收站”页按删除时间从新到旧列出被删日记并可恢复，正文、附件与标签关系都不动。
- 检索在一条 SQL 里组合三类条件：关键词对 `search_text`（标题与正文折叠空白、转小写后的副本）做转义后的子串匹配，标签走 `entry_tags` 交集，日期范围含两端；排序与分页与时间线一致。`search_text` 在每次保存时与 `excerpt` 同一次 UPDATE 重算，迁移 v4 时对旧库回填一次（见 [ADR-0007](../decisions/0007-search-strategy.md)）。
- 未来信件把开启时刻存成 UTC（`unlock_at_utc`）并另存写信时区（`unlock_timezone`）；三态在每次读取时按当前 UTC 推进，不依赖后台定时器；到期后原信冻结，回应独立写 `reply_json` / `reply_text`（见 [ADR-0008](../decisions/0008-letters-unlock-time.md)）。
- 导出把正文按节点映射成 Markdown、图片复制到目标目录的 images/ 并用相对路径引用；文件名是「日期-标题」，重名追加序号（见 [ADR-0009](../decisions/0009-markdown-export.md)）。导出由 Rust 完成，目录选择交给 dialog 插件，前端只传路径字符串。
- 备份是一个目录：`VACUUM INTO` 生成数据库快照，复制 `attachments/` 与 `thumbnails/`，最后写带 SHA-256 清单的 `manifest.json`。恢复先预检、再自动备份当前空间，然后把快照挂上来在一条事务里换掉五张表，附件目录在事务成功后替换（见 [ADR-0010](../decisions/0010-backup-format.md)）。
- “设置”页显示数据目录、数据库文件路径与模式版本，并提供 Markdown 导出、备份与恢复入口。

尚未接入：前端测试与 CI。导航中的“收藏册”仍是占位页。

## 目标分层

```text
React UI
  -> application services
    -> repositories / Tauri commands
      -> SQLite + local attachment directory
```

### 前端

```text
src/
  app/                应用壳、导航和页面入口
  components/         跨功能复用的视觉组件
  features/           按产品能力组织的页面和业务模块（journal、timeline、calendar、search、letters、trash、settings）
  lib/                日期、格式化、调用封装等基础能力
  types/              跨模块共享的 TypeScript 类型
  styles.css          全局设计令牌和基础样式
```

UI 不直接读写 SQLite 和文件系统。页面通过应用服务发起动作，应用服务再调用 Tauri 命令或仓储接口。

### Rust / Tauri

Rust 层负责本地数据边界：

- 初始化和迁移 SQLite。
- 原子写入日记、标签、附件元数据与信件，并维护软删除状态、搜索列和信件三态。
- 保存附件、生成缩略图和清理孤立文件。
- 把正文降级成 Markdown 并复制图片（导出），以及备份、校验和恢复。
- 备份与恢复由 `backup/` 模块负责：`create` 写快照与清单，`inspect` 校验，`restore` 换行与换目录。
- 只暴露经过类型约束的 Tauri 命令。

## 本地文件布局

```text
Memento Library/
  memento.db
  attachments/
  thumbnails/
  backups/        恢复前自动留下的空间备份
```

数据库保存结构化内容和附件元数据；原始图片不以 Base64 长期存储在数据库中。

M1 只使用 `memento.db`；M2-02 起首次导入图片时创建 `attachments/` 与 `thumbnails/`；M5-02 起恢复前会把当前空间写进 `backups/`。标签、软删除状态与搜索列都存在数据库里，不新增目录。导出产物与备份目录都在用户选的位置，不预建空目录。

## 时间处理

- `entry_date` 表示用户看到的本地日历日期，格式为 `YYYY-MM-DD`，由前端按本地日历计算后传给命令，不随修改或时区变化。
- `created_at_utc`、`updated_at_utc`、`deleted_at_utc`、`opened_at_utc` 和信件开启时间保存 UTC 时间戳（RFC 3339）。
- 未来信件额外保存创建时的时区（IANA 名称），用于跨时区时解释用户原本的意图；时区换算在前端按本机时区完成，Rust 不引时区库。

## 依赖边界

第一版只加入能够形成闭环的依赖。编辑器、SQLite、备份和应用锁分别引入，并在对应 ADR 中记录原因和迁移成本，避免在脚手架阶段提前锁定全部实现。

当前已引入：`rusqlite`（bundled）、`time`、`uuid`、`sha2`、`image` 0.25（关闭默认特性，只开 png 与 jpeg）、`tauri-plugin-dialog`（选目录与确认框）；前端已引入 Tiptap 3.32 作为正文编辑器与 `@tauri-apps/plugin-dialog`。
