/** 把选中的本地日历日期换成开启时刻：当天 00:00:00 按本机时区换算成 UTC。 */
export function unlockAtUtc(localDate: string): string {
  const [year, month, day] = localDate.split("-").map(Number);
  return new Date(year, month - 1, day, 0, 0, 0).toISOString();
}

/** 记录写信时的时区，只用于解释用户原本的意图，不参与开启时刻计算。 */
export function currentTimeZone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone;
}

/** 开启日期按写信时的时区读回本地日历日期，系统时区变了也不会挪一天。 */
export function unlockLocalDate(unlockAtUtc: string, timeZone: string): string {
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(new Date(unlockAtUtc));
  const pick = (type: string) => parts.find((part) => part.type === type)?.value ?? "";
  return pick("year") + "-" + pick("month") + "-" + pick("day");
}

/** 按写信时的时区显示开启时刻：换了系统时区也不会显示成前一天。 */
export function formatMomentInZone(unlockAtUtc: string, timeZone: string): string {
  return new Intl.DateTimeFormat("zh-CN", {
    timeZone,
    year: "numeric",
    month: "long",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(new Date(unlockAtUtc));
}

/** 距离开启还剩几个日历日：按写信时的时区数日子，十几秒的等待不会说成「还有 1 天」。 */
export function daysUntil(unlockAtUtc: string, timeZone: string, now: Date): number {
  const target = Date.parse(unlockLocalDate(unlockAtUtc, timeZone) + "T00:00:00Z");
  const today = Date.parse(unlockLocalDate(now.toISOString(), timeZone) + "T00:00:00Z");
  return Math.round((target - today) / 86_400_000);
}
