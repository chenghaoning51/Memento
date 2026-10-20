import { useEffect, type RefObject } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

/** 托盘「退出」：前端收到后先把当前页落盘，再真正结束进程。 */
const QUIT_REQUEST_EVENT = "memento://quit-requested";

/**
 * 桌面窗口的两个退出入口：
 * - 点击标题栏叉号只把窗口最小化，不结束进程；保存失败时窗口留在原处并提示。
 * - 托盘菜单的「退出」先落盘，成功后才真正结束，失败则把窗口叫回来。
 * 两者都走页面注册的 flush 守卫，与切页的保存承诺一致（A04、A05）。
 */
export function useDesktopLifecycle(leaveGuard: RefObject<(() => Promise<boolean>) | null>) {
  useEffect(() => {
    const window = getCurrentWindow();

    // 一律拦下销毁：叉号只是最小化，进程要留在托盘里。
    const unlistenClose = window.onCloseRequested(async (event) => {
      event.preventDefault();
      const guard = leaveGuard.current;
      // 落盘失败就把窗口留在原处，让用户看到“保存失败”和重试入口。
      if (guard && !(await guard())) return;
      await window.minimize();
    });

    const unlistenQuit = listen(QUIT_REQUEST_EVENT, async () => {
      const guard = leaveGuard.current;
      if (guard && !(await guard())) {
        await window.show();
        await window.unminimize();
        await window.setFocus();
        return;
      }
      await invoke("quit_app");
    });

    return () => {
      void unlistenClose.then((dispose) => dispose());
      void unlistenQuit.then((dispose) => dispose());
    };
  }, [leaveGuard]);
}
