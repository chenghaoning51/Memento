import { useCallback, useEffect, useRef, useState } from "react";
import { Clock3 } from "lucide-react";
import { attachmentSource } from "../../lib/attachmentSource";
import { listTimeline } from "../../lib/entryApi";
import { formatCalendarDate, formatClockTime } from "../../lib/localDate";
import type { EntrySummary } from "../../types/entry";

/** 一次取多少篇；滚动到底部再取下一批，避免一次把全部日记读进内存。 */
const PAGE_SIZE = 40;

type DayGroup = {
  entryDate: string;
  entries: EntrySummary[];
};

/** 按日期分组；同一天的多篇保持命令返回的顺序（当天从晚到早）。 */
function groupByDate(entries: EntrySummary[]): DayGroup[] {
  const groups: DayGroup[] = [];
  for (const entry of entries) {
    const last = groups[groups.length - 1];
    if (last && last.entryDate === entry.entryDate) {
      last.entries.push(entry);
    } else {
      groups.push({ entryDate: entry.entryDate, entries: [entry] });
    }
  }
  return groups;
}

type TimelinePageProps = {
  onOpenEntry: (entryDate: string, entryId: string) => void;
};

/** 时间线：按日期从新到旧翻阅全部日记，滚到底自动续读。 */
export function TimelinePage({ onOpenEntry }: TimelinePageProps) {
  const [entries, setEntries] = useState<EntrySummary[] | null>(null);
  const [hasMore, setHasMore] = useState(true);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const sentinelRef = useRef<HTMLDivElement | null>(null);
  const loadingRef = useRef(false);
  const entriesRef = useRef<EntrySummary[]>([]);

  const loadMore = useCallback(async () => {
    if (loadingRef.current) return;
    loadingRef.current = true;
    setLoading(true);
    try {
      // 用已读条数做游标：批次之间既不重复也不漏。
      const batch = await listTimeline(PAGE_SIZE, entriesRef.current.length);
      entriesRef.current = [...entriesRef.current, ...batch];
      setEntries(entriesRef.current);
      setHasMore(batch.length === PAGE_SIZE);
      setError(null);
    } catch (cause) {
      setError(String(cause));
    } finally {
      loadingRef.current = false;
      setLoading(false);
    }
  }, []);

  // 首次进入先取第一批。
  useEffect(() => {
    void loadMore();
  }, [loadMore]);

  // 底部哨兵进入视口时续读；没有更多就不再观察。
  useEffect(() => {
    const sentinel = sentinelRef.current;
    if (!sentinel || !hasMore) return;
    const observer = new IntersectionObserver((records) => {
      if (records.some((record) => record.isIntersecting)) void loadMore();
    });
    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [hasMore, loadMore, entries]);

  const groups = groupByDate(entries ?? []);

  return (
    <section className="timeline-page" aria-labelledby="page-title">
      <header className="page-header">
        <div>
          <p className="eyebrow">A PRIVATE JOURNAL</p>
          <h1 id="page-title">时间线</h1>
        </div>
        <span className="date-stamp">{entries && entries.length > 0 ? `已读 ${entries.length} 篇` : "从头读起"}</span>
      </header>

      {entries !== null && entries.length === 0 ? (
        <div className="section-empty">
          <span>还没有可以翻阅的记录。</span>
        </div>
      ) : (
        <div className="timeline">
          {groups.map((group) => (
            <section className="timeline-day" key={group.entryDate}>
              <h2 className="timeline-date">{formatCalendarDate(group.entryDate)}</h2>
              <ul className="timeline-entries">
                {group.entries.map((entry) => (
                  <li key={entry.id}>
                    <button
                      className={`timeline-entry${entry.coverAttachmentId ? " has-cover" : ""}`}
                      type="button"
                      onClick={() => onOpenEntry(entry.entryDate, entry.id)}
                    >
                      {entry.coverAttachmentId ? (
                        <img className="timeline-cover" src={attachmentSource(entry.coverAttachmentId)} alt="" />
                      ) : (
                        <span className="timeline-dot" aria-hidden="true" />
                      )}
                      <span className="timeline-excerpt">{entry.excerpt || "空白日记"}</span>
                      <span className="timeline-time">{formatClockTime(entry.createdAtUtc)}</span>
                    </button>
                  </li>
                ))}
              </ul>
            </section>
          ))}

          <div className="timeline-more" ref={sentinelRef}>
            {loading ? (
              <span>正在读取…</span>
            ) : hasMore ? (
              <span>继续向下滚动</span>
            ) : (
              <span>
                <Clock3 size={13} strokeWidth={1.6} aria-hidden="true" /> 到底了
              </span>
            )}
          </div>
        </div>
      )}

      {error && (
        <p className="page-error" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
