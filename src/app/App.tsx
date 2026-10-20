import { useCallback, useRef, useState } from "react";
import { AppShell } from "./AppShell";
import type { AppPage } from "./navigation";
import { useDesktopLifecycle } from "./useDesktopLifecycle";
import { useUnreadLetters } from "./useUnreadLetters";
import { CalendarPage } from "../features/calendar/CalendarPage";
import { JournalPage } from "../features/journal/JournalPage";
import { LettersPage } from "../features/letters/LettersPage";
import { SectionPage } from "../features/placeholder/SectionPage";
import { SearchPage } from "../features/search/SearchPage";
import { SettingsPage } from "../features/settings/SettingsPage";
import { TimelinePage } from "../features/timeline/TimelinePage";
import { TrashPage } from "../features/trash/TrashPage";

/** 从时间线或月历点进某一天时记下的来源，用于“返回”按钮与侧栏高亮。 */
type JournalTarget = {
  entryDate: string;
  entryId?: string;
  back: Extract<AppPage, "timeline" | "calendar" | "search">;
};

const backLabels: Record<JournalTarget["back"], string> = {
  timeline: "返回时间线",
  calendar: "返回时间地图",
  search: "返回搜索",
};

export default function App() {
  const [page, setPage] = useState<AppPage>("today");
  const [target, setTarget] = useState<JournalTarget | null>(null);
  // 从日记里的标签跳过来时，搜索页要带着这个标签打开。
  const [tagFilter, setTagFilter] = useState<string | undefined>(undefined);
  const leaveGuardRef = useRef<(() => Promise<boolean>) | null>(null);

  // 切页前先落盘：编辑中的页面可以注册一个守卫，落盘失败时取消切换（A04）。
  const navigate = useCallback(
    (next: AppPage) => {
      if (next === page && !target) return;
      void (async () => {
        if (leaveGuardRef.current && !(await leaveGuardRef.current())) return;
        setTarget(null);
        // 从侧栏直接进搜索时不带任何预选标签。
        if (next === "search") setTagFilter(undefined);
        setPage(next);
      })();
    },
    [page, target],
  );

  // 点日记里的标签：落到搜索页，并预先选中这个标签。
  const openTag = useCallback(
    (tagId: string) => {
      void (async () => {
        if (leaveGuardRef.current && !(await leaveGuardRef.current())) return;
        setTarget(null);
        setTagFilter(tagId);
        setPage("search");
      })();
    },
    [],
  );

  // 时间线与月历进入的是同一个日记页，只是带上日期和来源页。
  const openJournal = useCallback(
    (from: JournalTarget["back"], entryDate: string, entryId?: string) => {
      void (async () => {
        if (leaveGuardRef.current && !(await leaveGuardRef.current())) return;
        setTarget({ entryDate, entryId, back: from });
        setPage("today");
      })();
    },
    [],
  );

  const registerLeaveGuard = useCallback((guard: (() => Promise<boolean>) | null) => {
    leaveGuardRef.current = guard;
  }, []);

  // 叉号收进托盘、托盘「退出」才结束进程；两者都先走页面注册的落盘守卫。
  useDesktopLifecycle(leaveGuardRef);

  // 到期待读的信在侧栏留个角标；信箱页每次读写后回报一次，角标跟着变。
  const { count: unreadLetters, refresh: refreshUnreadLetters } = useUnreadLetters();

  return (
    <AppShell
      activePage={target?.back ?? page}
      onNavigate={navigate}
      letterBadge={unreadLetters}
    >
      {page === "today" && (
        <JournalPage
          entryDate={target?.entryDate}
          entryId={target?.entryId}
          back={target ? { label: backLabels[target.back], onClick: () => navigate(target.back) } : undefined}
          onOpenTag={openTag}
          registerLeaveGuard={registerLeaveGuard}
        />
      )}
      {page === "timeline" && (
        <TimelinePage
          onOpenEntry={(entryDate, entryId) => openJournal("timeline", entryDate, entryId)}
          onGoToday={() => navigate("today")}
        />
      )}
      {page === "calendar" && <CalendarPage onOpenDate={(entryDate) => openJournal("calendar", entryDate)} />}
      {page === "search" && (
        <SearchPage
          onOpenEntry={(entryDate, entryId) => openJournal("search", entryDate, entryId)}
          initialTagId={tagFilter}
        />
      )}
      {page === "letters" && <LettersPage onLettersChanged={refreshUnreadLetters} />}
      {page === "trash" && <TrashPage />}
      {page === "settings" && <SettingsPage />}
      {page !== "today" &&
        page !== "timeline" &&
        page !== "calendar" &&
        page !== "search" &&
        page !== "letters" &&
        page !== "trash" &&
        page !== "settings" && <SectionPage page={page} />}
    </AppShell>
  );
}
