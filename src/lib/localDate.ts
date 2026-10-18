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

/** 把 YYYY-MM-DD 还原成本地时区的当天零点，避免 UTC 解析把日期挪一天。 */
export function parseLocalDate(entryDate: string): Date {
  const [year, month, day] = entryDate.split("-").map(Number);
  return new Date(year, month - 1, day);
}

/** 把 YYYY-MM-DD 显示成中文日期。 */
export function formatCalendarDate(entryDate: string): string {
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "long",
    day: "numeric",
  }).format(parseLocalDate(entryDate));
}

/** 显示完整时刻：年份、日期与时分都按本机时区。 */
export function formatMoment(isoUtc: string): string {
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "long",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(new Date(isoUtc));
}

export function formatClockTime(isoUtc: string): string {
  return new Intl.DateTimeFormat("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(new Date(isoUtc));
}
