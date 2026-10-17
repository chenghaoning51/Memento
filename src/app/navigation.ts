import {
  Bookmark,
  CalendarDays,
  Clock3,
  MailOpen,
  Settings2,
  SunMedium,
  type LucideIcon,
} from "lucide-react";

export type AppPage = "today" | "timeline" | "calendar" | "letters" | "collection" | "settings";

export type NavigationItem = {
  id: AppPage;
  label: string;
  icon: LucideIcon;
};

export const primaryNavigation: NavigationItem[] = [
  { id: "today", label: "今天", icon: SunMedium },
  { id: "timeline", label: "时间线", icon: Clock3 },
  { id: "calendar", label: "时间地图", icon: CalendarDays },
  { id: "letters", label: "未来信箱", icon: MailOpen },
  { id: "collection", label: "收藏册", icon: Bookmark },
];

export const utilityNavigation: NavigationItem[] = [
  { id: "settings", label: "设置", icon: Settings2 },
];

export const pageTitles: Record<AppPage, string> = {
  today: "今天",
  timeline: "时间线",
  calendar: "时间地图",
  letters: "未来信箱",
  collection: "收藏册",
  settings: "设置",
};
