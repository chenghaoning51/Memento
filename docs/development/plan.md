# 开发计划与路线图

- 基线：2026-10-20，1.0.1 已交付。
- 产品范围见[产品](../product/README.md)与[功能规则](../product/features.md)。
- 完成依据见[验收清单](acceptance.md)，执行记录见[迭代记录](iterations.md)。

## 当前状态

M0–M6 全部完成，P0 发布门槛已满足，Windows 安装包 `Memento_1.0.1_x64-setup.exe` 已在本机验证。

| 阶段 | 交付 | 状态 |
| --- | --- | --- |
| M0 开发准备 | 产品场景细化、SQLite 与编辑器选型 ADR、验收数据与版本基线 | 已完成 |
| M1 可靠保存 | 数据目录与迁移、日记创建/编辑、自动保存、重开 | 已完成 |
| M2 日记闭环 | Tiptap 格式编辑、图片附件、标签、回收站 | 已完成 |
| M3 回看与检索 | 时间线、月历、日期详情、关键词/标签/日期筛选 | 已完成 |
| M4 未来信箱 | 写信、锁定、到期阅读、独立回应 | 已完成 |
| M5 导出与恢复 | Markdown 导出、完整备份、校验与恢复 | 已完成 |
| M6 Windows 发布 | 回归、CSP 与权限复核、安装与保留数据升级 | 已完成 |
| M6-04 标题补齐 | 日记标题输入与列表展示（M6 收尾时发现遗漏） | 已完成 |

已知缺口：

- 前端没有测试与 CI，回归依靠构建检查与真实桌面操作。
- 安装验证只覆盖同一台机器的覆盖安装，干净机器首次安装与其它 Windows 版本未验证。
- 主 JS 包约 690 kB，构建有 chunk 体积警告；桌面本地加载下无可感知影响。
- 恢复期间不禁止界面操作，极端并发没有互斥。
- 更换数据目录采用复制而不是移动：大空间会临时占用两份磁盘；旧目录需用户自行清理。

## 优先级

| 优先级 | 范围 |
| --- | --- |
| P0 已交付 | 本地存储与迁移、日记编辑与自动保存、图片与标签、回收站、时间线月历、搜索、未来信件、Markdown 导出、完整备份恢复、Windows 发布验证 |
| P1 稳定后评估 | 收藏卡片与主题册、月份切片、全局快速捕捉、最近阅读位置与草稿恢复、应用锁与凭据保护 |
| P2 暂不实施 | 自由画布与贴纸、云同步与移动端、多人协作、AI 整理、公开分享 |

P0 内部取舍顺序：数据不丢失与可恢复 → 一条可操作的完整流程 → 查找与回看 → 丰富展示。时间压力下先减少并行、收敛视觉细节，不能删除自动保存失败处理、图片完整性、回收站或备份恢复，也不能把占位入口记为完成。

## 架构现状

```text
React 页面与编辑状态
  → 应用服务（src/lib）
    → Tauri 命令（33 个）
      → Rust 数据与文件操作
        → SQLite + 独立附件目录
```

| 层 | 位置 | 现状 |
| --- | --- | --- |
| 前端入口 | `src/main.tsx` | 只挂载 React 与全局样式 |
| 应用壳 | `src/app/App.tsx`、`AppShell.tsx`、`navigation.ts`、`useDesktopLifecycle.ts` | 组件状态切页、切页守卫、来源返回、叉号收进托盘与托盘退出；“收藏册”是占位页 |
| 功能页面 | `src/features/{journal,timeline,calendar,search,letters,trash,settings}` | 全部接真实数据 |
| 应用服务 | `src/lib/*Api.ts`、`entryContent.ts`、`letterContent.ts`、`localDate.ts`、`unlockTime.ts`、`errorText.ts`、`locationApi.ts` | UI 不碰 SQL 与文件路径 |
| 编辑与保存 | `features/journal/useAutosave.ts`、`EntryEditor.tsx`、`TagBar.tsx` | 800 ms 空闲保存、5 秒强制保存、串行写入、失败重试；切页、叉号与托盘退出共用同一个 flush 守卫 |
| 桌面入口 | `src-tauri/src/lib.rs`、`tray.rs` | 打开 Library、注册 33 个命令、系统托盘、dialog 插件与 `attachment` 协议 |
| Rust 模块 | `db/`、`entries/`、`attachments/`、`tags/`、`letters/`、`export/`、`backup/`、`location.rs`、`tray.rs`、`commands.rs`、`error.rs` | 迁移 v1–v5、仓储、正文转 Markdown、备份与恢复、数据目录切换、托盘图标与菜单 |
| 权限与打包 | `src-tauri/capabilities/default.json`、`tauri.conf.json` | `core:default`、窗口隐藏/显示/还原/聚焦、`dialog:default`；CSP 只放行自身资源与附件图片；NSIS 目标 |

前端依赖：React 19、TypeScript 6、Vite 8、Tauri API 2、dialog 插件 2.8、Lucide、Tiptap 3.32。
Rust 依赖：Tauri 2（启用 `tray-icon`）、Serde、rusqlite（bundled）、time、uuid、sha2、image 0.25（只开 png/jpeg）、tauri-plugin-dialog。

## 实施前仍需的决策

| 事项 | 触发时点 | 产出 |
| --- | --- | --- |
| 永久删除与附件清理规则 | 实现该功能前 | 产品规则 + ADR；覆盖写入中断、软删除与恢复 |
| 收藏册的数据模型 | 进入 P1 前 | ADR；复用已有附件与标签能力 |
| 应用锁与加密方案 | 进入 P1 前 | ADR；明确威胁模型，不把本地保存说成加密 |
| 前端测试工具与 CI | 下次功能开发前 | ADR + 命令文档 |

## 主要风险

| 风险 | 应对 |
| --- | --- |
| 保存请求乱序、关闭或写入失败导致内容丢失 | 串行写入、flush 与切页守卫已实现；修改保存逻辑必须重跑 A03–A05 |
| 中文输入法与正文 JSON 演进损坏内容 | 正文按版本处理；关键格式往返验证；新格式先写 ADR |
| SQLite、正文引用与附件文件不一致 | 先写文件后插行、失败回滚文件；备份与回收站沿用同一完整性要求 |
| 日期与时区偏移 | 日历日期与 UTC 分开保存；跨午夜、换时区与夏令时已覆盖 |
| 备份生成成功但无法恢复 | 预检 + 事务换行 + 恢复前自动备份；变更备份格式必须重跑 A17–A19 |
| 中文子串搜索或大图卡顿 | 1,006 篇下已采样；先限制一次载入量，换索引前用证据决定 |
| 前端无自动化测试 | 高风险改动在迭代记录里保留手工步骤与证据 |

## 迭代方式

- 两周一个迭代，约 8 个有效开发日；每轮只有一个主目标，同一时间只推进一条未验收的链路。
- 开工前确认产品规则、架构依据与验收编号；结束时回归、核对证据并更新本文件与[迭代记录](iterations.md)。
- 完成定义、缺陷分级（S0–S3）与评审门槛见[迭代流程](iteration-workflow.md)。
