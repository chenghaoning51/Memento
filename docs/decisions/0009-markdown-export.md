# ADR-0009：导出用 Markdown 文件与相对路径图片

- 状态：Accepted
- 日期：2026-10-18

## 背景

M5-01 要把日记带出应用。产品文档已经定了产物形态（Markdown + 图片、相对路径引用、重名追加序号、目标不可写时报错），但没有定下三件实现层面的事：正文 JSON 怎么降级成 Markdown；图片文件叫什么名字；导出放在哪个进程里做、用什么方式让用户选目录。

约束来自已有实现：正文是 v2 的 ProseMirror JSON（`content_json`），纯文本副本 `content_text` 已经丢掉了加粗、链接、图片等结构，不能用来导出；图片以附件形式存在 `attachments/<entry_id>/<attachment_id>.<ext>`，正文 image 节点只记 `attachmentId`；前端不接触磁盘路径（架构约束），导出必须由 Rust 完成。

## 决定

- 导出由 Rust 完成：新增 `export` 模块，`write_entries` 接收目标目录、Library 根、日记列表与「附件 ID → 相对路径」映射，写出 Markdown 并复制图片。
- 正文转换手写：遍历 v2 文档节点，按 `doc`/`paragraph`/`heading`/`blockquote`/`horizontalRule`/`hardBreak`/`text`/`image` 映射到 Markdown；行内 `bold`/`italic`/`strike`/`code`/`link` 走 marks。不认识的节点递归取其子节点，纯文本兜底。
- 图片复制到目标目录的 `images/`，文件名沿用附件自己的文件名（`<attachment_id>.<ext>`，天然唯一），正文写 `![alt](images/<文件名>)`。同一张图在多次导出里被引用只复制一次。
- 文件名是「日期-标题」，标题为空时用摘要前 16 个字，再空就叫「无题」；`\ / : * ? " < > |` 与控制字符换成空格，连续空白折叠，去掉结尾的点，截到 60 字。同一次导出内遇到同名追加「 (2)」「 (3)」；目标目录里已有同名文件也追加序号，不覆盖。
- 目录选择交给 `tauri-plugin-dialog`：前端调用 `open({ directory: true })` 拿到用户选的路径，再把路径字符串传给 `export_entries`。用户取消时返回 null，前端不发起导出。
- 导出成功返回 `ExportSummary { directory, entryCount, imageCount }`，前端显示在设置页。

## 原因

- 手写转换而不是引入 Markdown 序列化库：需要支持的节点就是编辑器工具栏能产出的那几种（见 [ADR-0004](0004-tiptap-editor-and-content-v2.md)），一个库要覆盖 ProseMirror 到 Markdown 的映射仍要自己写规则，还会带来新的依赖与版本约束。手写的映射表是可读的、可测的，新增节点时加一条分支即可。
- 用附件自己的文件名做图片名：附件 ID 是 UUID，天然唯一，不需要额外的重名处理；反过来说，用原文件名（如「截图 2026-10-17.png」）会在多篇日记里撞名，还要维护一张映射表。代价是导出目录里的图片名对人不友好——但用户看的是 Markdown 里的图片，不是文件名。
- 目录选择用 dialog 插件而不是自己调 Win32：插件已经处理了父窗口、COM 初始化与取消路径，Windows 与将来的平台共用一套调用；自己写要维护 `IFileOpenDialog` 的线程模型。
- 选择路径由前端传字符串而不是让 Rust 弹窗：Rust 侧的同步命令在 IPC 线程池里跑，直接弹模态对话框会阻塞该线程；把弹窗交给插件的异步命令，用户看到的是非阻塞的系统对话框。
- 导出范围先做「全部未删除的日记」：勾选部分日记需要在界面上先有选择模型，而当前列表是「当天 + 时间线」两套视图，选择状态无处安放。全部导出已经能满足「把内容带走」的诉求。

## 备选方案

- **用 `content_text` 直接写 Markdown**：实现最省事，但加粗、链接、图片、引用全部丢失，导出的东西和用户写的不是一回事。
- **导出成 HTML 或 PDF**：可读性更好，但用户后续要编辑、要进 Obsidian 一类的工具，Markdown 才是通用格式；PDF 还要引入排版依赖。
- **图片按原文件名复制**：对人更友好，但多篇日记引用同名文件时会互相覆盖或需要加序号，正文里的路径也要跟着改。
- **一次导出打成一个 zip**：便于传输，但用户拿到后还要解压才能看；产品文档写的是「写入用户选择的目录」，保持目录形态。
- **让 Rust 弹系统对话框**：少一次 IPC 往返，但会占住 IPC 线程，且要自己处理父窗口句柄与 COM。

## 影响

- 新增依赖：`tauri-plugin-dialog`（Rust 侧 2.6.0）与 `@tauri-apps/plugin-dialog`（前端 2.8.3）。capability 增加 `dialog:default` 权限。
- 新增模块 `src-tauri/src/export/`（`mod.rs` 渲染与写盘、`repository.rs` 取日记与附件路径）与命令 `export_entries`，命令总数变为二十五个。
- 设置页新增「导出」区块：按钮 + 结果行；导出失败时显示 `page-error`。
- 导出不写数据库，不改任何状态；重复导出只是产生带序号的新文件。
- 回收站、标签、信件不进入 Markdown 导出：标签没有对应的 Markdown 语义，信件属于另一种内容形态，两者都留到备份（M5-02）里完整保留。
- 图片只导出原图，不导出缩略图：缩略图是派生数据，Markdown 里引用的应该是能看清的原图。
