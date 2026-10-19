# 开发环境

## 前置条件

- Node.js 22 或兼容的 LTS 版本。
- pnpm 11。
- Rust stable 和 Cargo。
- Windows 桌面开发所需的 WebView2 与 Visual Studio C++ Build Tools。

## 安装依赖

```bash
pnpm install
```

## 常用命令

```bash
# 只启动 Vite 前端
pnpm dev

# 启动完整桌面应用
pnpm tauri dev

# 类型检查并构建前端
pnpm build

# 检查 Rust 依赖和代码
cargo check --manifest-path src-tauri/Cargo.toml

# 运行 Rust 测试（迁移、持久化、附件、检索、信件、导出、备份恢复）
cargo test --manifest-path src-tauri/Cargo.toml

# 检查已跟踪变更的空白问题
git diff --check

# 构建 Windows 安装包（NSIS）
pnpm tauri build
```

安装包输出到 `src-tauri/target/release/bundle/nsis/Memento_<版本>_x64-setup.exe`；只配置了 NSIS，不产出 MSI。

前端还没有测试命令，也没有 CI；前端行为依靠构建检查加真实桌面验收。Rust 侧有 `cargo test`（13 个用例），覆盖迁移、持久化、附件读写、标签、检索、信件状态、导出降级与备份恢复这类无法靠肉眼确认的行为。后续给前端引入测试工具时先写 ADR，再把实际可运行命令同步到本文和根 README；不要把规划中的命令当作已存在。

发布加固的约定：CSP 只放行自身资源、`ipc` 与 `http://attachment.localhost` 的图片，新增外部资源前先确认是否真的需要；前端错误一律经 `src/lib/errorText.ts` 转成用户可读的一句话；前端 `@tauri-apps/plugin-dialog` 必须与 Rust `tauri-plugin-dialog` 同版本，否则 `pnpm tauri build` 会拒绝构建。

## 第一次启动

1. 运行 `pnpm install`。
2. 运行 `pnpm tauri dev`。
3. 确认窗口标题为 `Memento · 私人时光收藏室`，并能看到左侧导航和“今天”工作台。
4. 在“今天”页写一段中文正文，等底部出现“已保存”，关闭并重开应用后内容仍在。

## 数据目录约定

首次启动时 Rust 侧在应用数据目录下创建 `Memento Library/`，其中 `memento.db` 是唯一的业务数据库，`attachments/` 与 `thumbnails/` 存原图与缩略图，`backups/` 存恢复前自动留下的空间备份。Windows 上应用数据目录为 `%APPDATA%\com.memento.app`，因此实际路径是：

```text
%APPDATA%\com.memento.app\Memento Library\memento.db
```

应用内的“设置”页直接显示解析后的完整路径与数据库模式版本，不要靠记忆猜测。删除该目录等同于清空所有日记；调试时可以先退出应用再备份或删除，避免在运行中直接改库文件。导出与备份的目标目录由用户在系统对话框里选，默认不在数据目录内。

用户可选的独立 Library 目录仍未实现：它需要单独的迁移与切换设计，不属于 M5 的完整空间备份恢复，留到后续评估。

## 开始开发前

1. 阅读[开发计划](plan.md)，确认本轮所属阶段和前置条件。
2. 按[迭代流程](iteration-workflow.md)细化任务，标注[验收编号](acceptance.md)。
3. 产品行为先写产品文档，数据或技术选择先更新架构与 ADR；新依赖决定完成后再接入。
4. 每阶段在真实桌面环境演示和保存验收证据，达到退出门槛后才更新路线图。
