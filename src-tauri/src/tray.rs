//! 系统托盘：关闭窗口只是最小化，真正退出由托盘菜单决定。
//!
//! 托盘「退出」不直接结束进程，而是先发事件让前端把未保存的内容写完；
//! 前端确认落盘后才调用 `quit_app`。这样 M1 的 flush 承诺在托盘退出路径上同样成立。

use tauri::menu::MenuBuilder;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

const TRAY_ID: &str = "memento-tray";
const MENU_SHOW: &str = "tray-show";
const MENU_QUIT: &str = "tray-quit";

/// 托盘点了「退出」，前端收到后先落盘再回调 `quit_app`。
const QUIT_REQUEST_EVENT: &str = "memento://quit-requested";

/// 挂上托盘图标与菜单。托盘图标复用窗口图标，不额外引入图片资源。
pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let menu = MenuBuilder::new(app)
        .text(MENU_SHOW, "打开主界面")
        .separator()
        .text(MENU_QUIT, "退出 Memento")
        .build()?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Memento · 私人时光收藏室")
        .menu(&menu)
        // 左键直接回到应用，菜单留给右键，与 Windows 托盘的习惯一致。
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_SHOW => show_main_window(app),
            MENU_QUIT => {
                let _ = app.emit(QUIT_REQUEST_EVENT, ());
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder.build(app)?;
    Ok(())
}

/// 把窗口叫回来：最小化状态要先还原，再置于前台。
fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
