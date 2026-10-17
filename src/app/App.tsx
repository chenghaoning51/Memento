import { useCallback, useRef, useState } from "react";
import { AppShell } from "./AppShell";
import type { AppPage } from "./navigation";
import { SectionPage } from "../features/placeholder/SectionPage";
import { SettingsPage } from "../features/settings/SettingsPage";
import { TodayPage } from "../features/today/TodayPage";
import { TrashPage } from "../features/trash/TrashPage";

export default function App() {
  const [page, setPage] = useState<AppPage>("today");
  const leaveGuardRef = useRef<(() => Promise<boolean>) | null>(null);

  // 切页前先落盘：编辑中的页面可以注册一个守卫，落盘失败时取消切换（A04）。
  const navigate = useCallback(
    (next: AppPage) => {
      if (next === page) return;
      void (async () => {
        if (leaveGuardRef.current && !(await leaveGuardRef.current())) return;
        setPage(next);
      })();
    },
    [page],
  );

  const registerLeaveGuard = useCallback((guard: (() => Promise<boolean>) | null) => {
    leaveGuardRef.current = guard;
  }, []);

  return (
    <AppShell activePage={page} onNavigate={navigate}>
      {page === "today" && <TodayPage registerLeaveGuard={registerLeaveGuard} />}
      {page === "trash" && <TrashPage />}
      {page === "settings" && <SettingsPage />}
      {page !== "today" && page !== "trash" && page !== "settings" && <SectionPage page={page} />}
    </AppShell>
  );
}
