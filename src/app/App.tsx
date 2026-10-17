import { useState } from "react";
import { AppShell } from "./AppShell";
import type { AppPage } from "./navigation";
import { SectionPage } from "../features/placeholder/SectionPage";
import { SettingsPage } from "../features/settings/SettingsPage";
import { TodayPage } from "../features/today/TodayPage";
import { TrashPage } from "../features/trash/TrashPage";

export default function App() {
  const [page, setPage] = useState<AppPage>("today");

  return (
    <AppShell activePage={page} onNavigate={setPage}>
      {page === "today" && <TodayPage />}
      {page === "trash" && <TrashPage />}
      {page === "settings" && <SettingsPage />}
      {page !== "today" && page !== "trash" && page !== "settings" && <SectionPage page={page} />}
    </AppShell>
  );
}
