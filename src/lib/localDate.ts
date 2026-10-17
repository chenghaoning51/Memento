/** 日记所属日期使用本地日历日期，不受 UTC 偏移影响。 */
export function toLocalDateString(date: Date): string {
  const year = String(date.getFullYear());
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function formatLocalDate(date: Date): string {
  return new Intl.DateTimeFormat("zh-CN", {
    weekday: "long",
    year: "numeric",
    month: "long",
    day: "numeric",
  }).format(date);
}

export function formatClockTime(isoUtc: string): string {
  return new Intl.DateTimeFormat("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(new Date(isoUtc));
}
