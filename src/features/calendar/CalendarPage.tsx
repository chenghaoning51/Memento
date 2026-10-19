import { useCallback, useEffect, useState } from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { attachmentSource } from "../../lib/attachmentSource";
import { listEntryDates } from "../../lib/entryApi";
import { parseLocalDate, toLocalDateString } from "../../lib/localDate";
import type { EntryDateSummary } from "../../types/entry";
import { errorText } from "../../lib/errorText";

const WEEKDAYS = ["一", "二", "三", "四", "五", "六", "日"];

/** 把本地日期加减若干天；用 Date 的本地构造，跨月跨年与闰日都由它处理。 */
function shiftDays(date: Date, days: number): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate() + days);
}

/** 月历格子：从当月第一天所在周的周一铺到下月最后一天所在周的周日。 */
function buildGrid(year: number, month: number): Date[] {
  const first = new Date(year, month, 1);
  const offset = (first.getDay() + 6) % 7;
  const start = shiftDays(first, -offset);
  const last = new Date(year, month + 1, 0);
  const total = offset + last.getDate();
  const cells = Math.ceil(total / 7) * 7;
  return Array.from({ length: cells }, (_, index) => shiftDays(start, index));
}

type CalendarPageProps = {
  onOpenDate: (entryDate: string) => void;
};

/** 时间地图：按月看有记录的日期，点某一天进入那天的日记。 */
export function CalendarPage({ onOpenDate }: CalendarPageProps) {
  const [cursor, setCursor] = useState(() => {
    const now = new Date();
    return { year: now.getFullYear(), month: now.getMonth() };
  });
  const [days, setDays] = useState<EntryDateSummary[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async (year: number, month: number) => {
    const grid = buildGrid(year, month);
    try {
      const summaries = await listEntryDates(
        toLocalDateString(grid[0]),
        toLocalDateString(grid[grid.length - 1]),
      );
      setDays(summaries);
      setError(null);
    } catch (cause) {
      setError(errorText(cause));
    }
  }, []);

  useEffect(() => {
    void load(cursor.year, cursor.month);
  }, [cursor, load]);

  const byDate = new Map((days ?? []).map((day) => [day.entryDate, day]));
  const grid = buildGrid(cursor.year, cursor.month);
  const monthLabel = new Intl.DateTimeFormat("zh-CN", { year: "numeric", month: "long" }).format(
    new Date(cursor.year, cursor.month, 1),
  );
  const monthTotal = (days ?? [])
    .filter((day) => parseLocalDate(day.entryDate).getMonth() === cursor.month)
    .reduce((sum, day) => sum + day.entryCount, 0);

  const step = (delta: number) => {
    setDays(null);
    setCursor((current) => {
      const next = new Date(current.year, current.month + delta, 1);
      return { year: next.getFullYear(), month: next.getMonth() };
    });
  };

  return (
    <section className="calendar-page" aria-labelledby="page-title">
      <header className="page-header">
        <div>
          <h1 id="page-title">时间地图</h1>
        </div>
        <span className="date-stamp">{monthTotal > 0 ? `这个月 ${monthTotal} 篇` : "这个月还是空白"}</span>
      </header>

      <div className="calendar">
        <div className="calendar-head">
          <button className="icon-button" type="button" onClick={() => step(-1)} title="上个月">
            <ChevronLeft size={16} strokeWidth={1.8} aria-hidden="true" />
            <span className="visually-hidden">上个月</span>
          </button>
          <h2 className="calendar-month">{monthLabel}</h2>
          <button className="icon-button" type="button" onClick={() => step(1)} title="下个月">
            <ChevronRight size={16} strokeWidth={1.8} aria-hidden="true" />
            <span className="visually-hidden">下个月</span>
          </button>
        </div>

        <div className="calendar-weekdays" aria-hidden="true">
          {WEEKDAYS.map((label) => (
            <span key={label}>{label}</span>
          ))}
        </div>

        <div className="calendar-grid">
          {grid.map((date) => {
            const entryDate = toLocalDateString(date);
            const summary = byDate.get(entryDate);
            const outside = date.getMonth() !== cursor.month;
            const classes = [
              "calendar-cell",
              outside ? "is-outside" : "",
              summary ? "has-entries" : "",
              entryDate === toLocalDateString(new Date()) ? "is-today" : "",
            ]
              .filter(Boolean)
              .join(" ");
            return (
              <button
                className={classes}
                key={entryDate}
                type="button"
                onClick={() => onOpenDate(entryDate)}
                title={summary ? `${summary.entryCount} 篇日记` : "这一天没有记录"}
              >
                {summary?.coverAttachmentId && (
                  <img className="calendar-cover" src={attachmentSource(summary.coverAttachmentId)} alt="" />
                )}
                <span className="calendar-day">{date.getDate()}</span>
                {summary && <span className="calendar-count">{summary.entryCount} 篇</span>}
              </button>
            );
          })}
        </div>
      </div>

      {error && (
        <p className="page-error" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
