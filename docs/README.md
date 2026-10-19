# Memento 文档

Memento 是 Windows 优先、离线可用的私人时光收藏室。1.0.0 已交付，M0–M6 全部完成。

## 阅读顺序

1. [产品](product/README.md)：为什么做、范围与优先级、设计原则。
2. [功能规则](product/features.md)：日记、时间回看、搜索、未来信件、导出备份恢复的行为。
3. [系统架构](architecture/overview.md)与[数据模型](architecture/data-model.md)。
4. [开发计划](development/plan.md)：当前状态、架构现状、风险与待决事项。
5. [开发环境](development/getting-started.md)：命令、数据目录、发布约定。
6. [迭代流程](development/iteration-workflow.md)：节奏、完成定义、代码约定与缺陷分级。
7. [验收清单](development/acceptance.md)：A01–A23 与发布门槛。
8. [迭代记录](development/iterations.md)：每轮的环境、场景与缺陷。
9. [架构决策](decisions/README.md)与[变更记录](changelog.md)。

## 文档分区

| 目录 | 用途 | 什么时候更新 |
| --- | --- | --- |
| `product/` | 愿景、范围与功能规则 | 用户流程或产品范围变化时 |
| `architecture/` | 模块边界、数据模型和基础设施 | 技术实现或数据契约变化时 |
| `development/` | 环境、命令、计划、流程与验收 | 开发流程、阶段安排或质量门槛变化时 |
| `decisions/` | 不容易逆转的技术和产品决策 | 选择方案或改变既有方案时 |
| `templates/` | ADR 模板 | 新增决策记录时 |

## 文档约定

- 产品行为只写在 `product/`，实现与数据契约只写在 `architecture/`，流程与验收只写在 `development/`；同一件事不重复叙述。
- 未实现的能力标为“计划中”或“占位”，不把占位页面写成已交付。
- 代码改动与文档改动放在同一个提交；用户可见的变化写进 [changelog](changelog.md)。
- 跨模块约定、依赖或格式变化先写 ADR。
- 术语固定：`日记` 是按日期组织的记录，`收藏` 是独立保存的片段（P1），`未来信件` 是按日期开启的内容。
