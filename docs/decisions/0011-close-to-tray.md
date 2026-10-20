# ADR-0011：叉号最小化到托盘，退出由托盘菜单发起

- 状态：Accepted
- 日期：2026-10-20

## 背景

1.0.0 的叉号直接退出进程。用户希望关窗不等于结束：点是叉号只收起窗口，进程留在托盘，需要时再从托盘手动退出。这带来三个问题：

- 谁来承担「关闭前先落盘」的承诺。改动前这段逻辑写在 `useAutosave` 里，只在该 Hook 挂载时（即“今天”页）生效。
- 最小化之后没有入口回到窗口，必须同时提供托盘图标。
- 托盘「退出」不能绕过 flush，否则就比叉号更容易丢内容。

另外 `tauri.conf.json` 没有注册托盘，Tauri 默认未启用 `tray-icon` 特性。

## 决定

- 叉号改为**最小化**：前端在 `onCloseRequested` 里一律 `preventDefault()`，flush 通过后调用 `window.minimize()`。窗口不会被销毁，任务栏与托盘里都还留着入口，进程继续运行。
- 系统托盘在 `setup` 阶段创建（`src-tauri/src/tray.rs`），托盘图标复用窗口图标（`app.default_window_icon()`），不新增图片资源。左键点图标回到主界面，右键菜单是「打开主界面」「退出 Memento」。
- 托盘「退出」不直接 `app.exit()`：Rust 只发出 `memento://quit-requested` 事件，前端落盘成功后再调用新增的 `quit_app` 命令结束进程；失败则 `show` + `unminimize` + `setFocus` 把窗口叫回来。
- 关闭与退出的 flush 统一由 `src/app/useDesktopLifecycle.ts` 负责，读取 `App` 里已有的 `leaveGuardRef`。`useAutosave` 不再自己监听关闭事件，避免两处逻辑在是否挂载时行为不一致。
- `tauri` 启用 `tray-icon` 特性；能力文件补充 `core:window:allow-minimize`、`allow-unminimize`、`allow-show`、`allow-set-focus`。`allow-destroy` 不再需要，移除。

## 原因

- 把 flush 提到应用壳层：改动前关闭逻辑挂在日记页，切到搜索或设置再关窗就没有 flush。提到 `App` 后，「当前页面是否有未保存内容」这件事始终由页面自己注册的守卫决定，与切页共用同一个入口。
- 托盘退出走「事件 → 前端落盘 → 命令退出」而不是 Rust 直接退出：只有前端知道当前草稿内容，Rust 侧没有可 flush 的状态。事件往返多一次 IPC，但保住了 A05 的承诺。
- 复用窗口图标：`tauri.conf.json` 的 `bundle.icon` 已经带了 32×32 与 128×128，`default_window_icon()` 在 Windows 上直接读嵌入的图标资源，不需要额外的托盘图片，也不需要 `image-png` 特性来解码 PNG。
- 叉号在 flush 失败时**不最小化**：如果最小化，用户只会看到窗口消失，而不知道是因为保存失败；留在原处才能看到“保存失败”和「重试保存」。
- 右键出菜单、左键回窗口：符合 Windows 托管的习惯；`show_menu_on_left_click(false)` 是为此设置的。

## 备选方案

- **叉号直接 `hide()`**：更接近“收进托盘”的语义，但任务栏里也没有窗口了，用户只能通过托盘找回。最小化更符合用户描述的“最小化”，任务栏仍保留入口。
- **Rust 侧拦截 `CloseRequested` 做最小化**：`tauri::WindowEvent::CloseRequested` 无法取消（可取消的 `api` 只在前端事件的路径上），Rust 侧只能监听后重新 `show`，会出现窗口闪一下。
- **托盘退出直接 `app.exit(0)`**：实现最简单，但会绕过 flush，等于把“退出”变成比叉号更危险的操作。
- **给托盘加单独的 PNG 图标资源**：需要开启 `image-png` 特性并把图片打进二进制；窗口图标已经够用，没必要增加资源。

## 影响

- 新增 `src-tauri/src/tray.rs` 与 `src/app/useDesktopLifecycle.ts`；新增 Tauri 命令 `quit_app`，命令总数变为 33 个。
- `useAutosave` 只保留空闲保存与 `flush` 能力，不再监听 `onCloseRequested`。
- `capabilities/default.json` 新增四个窗口权限、移除 `allow-destroy`；`Cargo.toml` 的 `tauri` 增加 `tray-icon`。
- 关闭窗口不再是一个可靠的数据边界：窗口最小化后进程继续运行，数据库连接保持打开。原来的 A01、A02、A05 里“关闭并重开”的步骤仍然有效（最小化后手动从托盘退出再启动）。
- 托盘图标在应用运行期间必须存在，否则用户最小化后就找不回窗口；`setup` 里创建失败会让启动直接失败，而不是静默降级。
