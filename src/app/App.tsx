import { useState } from "react";
import { AppShell } from "./AppShell";
import type { AppPage } from "./navigation";
import { SectionPage } from "../features/placeholder/SectionPage";
import { SettingsPage } from "../features/settings/SettingsPage";
import { TodayPage } from "../features/today/TodayPage";

export default function App() {
  const [page, setPage] = useState<AppPage>("today");

  return (
    <AppShell activePage={page} onNavigate={setPage}>
      {page === "today" && <TodayPage />}
      {page === "settings" && <SettingsPage />}
      {page !== "today" && page !== "settings" && <SectionPage page={page} />}
    </AppShell>
  );
}
