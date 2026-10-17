# Memento 工作约定

## 先读文档

开始修改前先阅读：

- `docs/README.md`
- 与本次修改相关的产品或架构文档
- `docs/decisions/README.md`，判断是否需要新增 ADR

## 文档优先

- 产品行为变化先更新 `docs/product/`。
- 数据、模块边界或技术选型变化先更新 `docs/architecture/`。
- 开发流程和命令变化更新 `docs/development/` 和根目录 `README.md`。
- 每个用户可见的功能变更都要在 `docs/changelog.md` 留下记录。
- 不要把计划中的占位功能写成已实现功能。

## 实现约束

- 前端按 `app`、`features`、`components` 和 `lib` 组织，避免把所有逻辑堆在入口文件。
- UI 不直接访问 SQLite 或文件系统；通过应用服务和 Tauri 命令访问数据。
- 日记正文、附件和索引分开建模，附件不以 Base64 长期存入数据库。
- 时间戳保存 UTC，日记所属日期保存本地日历日期。
- 引入新的跨模块约定或依赖前，新增一条架构决策记录。

## 验证命令

```bash
pnpm build
cargo check --manifest-path src-tauri/Cargo.toml
```
