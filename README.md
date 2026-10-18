# Memento

Memento 是一个 Windows 优先、离线可用的私人时光收藏室。它把日记、照片、未来信件和生活片段放在一个本地空间里，让内容在时间里慢慢积累，而不是把记录变成连续打卡。

M0 至 M5 已完成，P0 功能齐备：

- **写**：按日期创建日记，正文用 Tiptap 编辑（标题、引用、分隔线、链接、加粗、斜体、撤销重做），可插入 PNG/JPEG 图片——原图复制进数据目录，移动或删除源文件后仍可读。停止输入 800 ms 自动保存，切页与关闭前先落盘，保存失败留在原页并提示。
- **找**：时间线按日期倒序分批加载，时间地图按月看哪天写过、几篇与首图，搜索支持关键词、标签与日期范围的组合筛选；日记可以打标签，删除先进回收站、可随时恢复。
- **信**：未来信箱可以写给以后的自己。开启时刻按写信时区换算成 UTC 保存，到期前正文在应用里保持关闭（这不是加密，只是给自己的约定），到期后重读原信并留下独立保存的回应；换时区或经过夏令时都不会让约定好的时刻漂移。
- **带走**：设置页可以把全部日记导出成 Markdown 与图片（相对路径引用，导出后不依赖 Memento），也可以备份整个空间——数据库一致性快照、原图、缩略图与带 SHA-256 清单的 manifest。恢复前先校验并展示内容，再自动备份当前空间，最后在一条事务里换掉数据；失败时当前空间保持可打开。

“今天”“时间线”“时间地图”“搜索”“未来信箱”“回收站”和“设置”是真实页面，只剩“收藏册”仍是占位。

## 快速开始

```bash
pnpm install
pnpm dev
pnpm tauri dev
```

构建前端：

```bash
pnpm build
```

评审变更前检查：

```bash
pnpm build
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
```

前端尚未配置测试命令，也没有 CI；构建成功只证明代码可构建，交互行为仍需按验收清单在真实桌面验证。每轮的验收结果记录在 [docs/development/iterations/](docs/development/iterations/) 下（迭代 001 至 009）。

## 开发安排

按 M0–M6 推进：开发准备与关键决策 → 可靠保存 → 编辑与附件 → 时间回看与检索 → 未来信箱 → 导出与恢复 → Windows 发布。M0 到 M5 已完成，P0 功能齐备；M6 的发布加固与 Windows 安装验收是下一步。

采用两周迭代，以真实桌面流程和验收证据判断完成。暂按 1 名开发者、每周 4 个有效开发日估算，含缓冲约 11–16 周，M1 完成后重新估算；该估算不是确定交付日期。每轮任务与实际结果依照迭代流程记录，不把占位页当作已交付功能。

- [MVP 开发计划](docs/development/development-plan.md)：当前基线、优先级、阶段任务、依赖与风险。
- [迭代与交付流程](docs/development/iteration-workflow.md)：开发节奏、开工/完成条件、缺陷分级与记录方式。
- [MVP 验收清单](docs/development/acceptance.md)：测试数据、阶段验收与发布门槛。

## 文档

- [文档总览](docs/README.md)
- [产品愿景](docs/product/vision.md)
- [MVP 范围](docs/product/mvp-scope.md)
- [时间回看](docs/product/time-browsing.md)
- [搜索与筛选](docs/product/search.md)
- [系统架构](docs/architecture/overview.md)
- [数据模型](docs/architecture/data-model.md)
- [开发环境](docs/development/getting-started.md)
- [开发约定](docs/development/conventions.md)
- [产品路线图](docs/roadmap.md)
- [变更记录](docs/changelog.md)
- [架构决策记录](docs/decisions/README.md)

## 仓库结构

```text
src/                  React 前端
  app/                应用壳和导航
  features/           按产品功能组织的页面和模块
  lib/                命令调用封装、日期与正文格式与版本升级
  types/              跨模块共享类型
  styles.css          全局设计令牌和基础样式
src-tauri/src/        Tauri 命令、SQLite 迁移、仓储与错误
docs/                 产品、架构、开发和决策文档
```

## 当前原则

1. 本地数据是默认数据，离线时核心体验仍然可用。
2. 先保证可靠保存、恢复和导出，再增加装饰和智能能力。
3. 产品不使用连续打卡、点赞或公开社交来驱动记录。
4. 规划中的能力会明确标记为计划，不把占位页面当成已完成能力。
