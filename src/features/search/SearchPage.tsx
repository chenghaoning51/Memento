import { useCallback, useEffect, useRef, useState } from "react";
import { Search as SearchIcon } from "lucide-react";
import { attachmentSource } from "../../lib/attachmentSource";
import { listTags, searchEntries } from "../../lib/entryApi";
import { formatCalendarDate, formatClockTime } from "../../lib/localDate";
import type { EntrySummary, SearchQuery, Tag } from "../../types/entry";
import { errorText } from "../../lib/errorText";

/** 一次取多少条；滚动到底部再取下一批，与时间线一致。 */
const PAGE_SIZE = 40;

type SearchPageProps = {
  onOpenEntry: (entryDate: string, entryId: string) => void;
};

/** 搜索：关键词、标签与日期范围可以单独用，也可以叠在一起。 */
export function SearchPage({ onOpenEntry }: SearchPageProps) {
  const [text, setText] = useState("");
  const [tags, setTags] = useState<Tag[]>([]);
  const [selectedTagIds, setSelectedTagIds] = useState<string[]>([]);
  const [startDate, setStartDate] = useState("");
  const [endDate, setEndDate] = useState("");

  const [results, setResults] = useState<EntrySummary[] | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadingRef = useRef(false);
  const resultsRef = useRef<EntrySummary[]>([]);
  const sentinelRef = useRef<HTMLDivElement | null>(null);
  // 每次条件变化都换一个代号，回来晚的旧请求不覆盖新结果（A12）。
  const requestRef = useRef(0);

  useEffect(() => {
    listTags()
      .then(setTags)
      .catch((cause) => setError(errorText(cause)));
  }, []);

  const runSearch = useCallback(
    async (query: SearchQuery, append: boolean) => {
      // 追加批次可以跳过，改条件必须重查：新请求换代号，在途的旧批次作废。
      if (append && loadingRef.current) return;
      loadingRef.current = true;
      const request = ++requestRef.current;
      setLoading(true);
      try {
        const batch = await searchEntries(query);
        if (request !== requestRef.current) return;
        resultsRef.current = append ? [...resultsRef.current, ...batch] : batch;
        setResults(resultsRef.current);
        setHasMore(batch.length === PAGE_SIZE);
        setError(null);
      } catch (cause) {
        if (request === requestRef.current) setError(errorText(cause));
      } finally {
        if (request === requestRef.current) {
          loadingRef.current = false;
          setLoading(false);
        }
      }
    },
    [],
  );

  // 条件一变就重查；第一次进入也先读一遍，好让空态和“还没搜过”区分开。
  useEffect(() => {
    resultsRef.current = [];
    void runSearch({ text, tagIds: selectedTagIds, startDate, endDate, limit: PAGE_SIZE, offset: 0 }, false);
  }, [text, selectedTagIds, startDate, endDate, runSearch]);

  const loadMore = useCallback(() => {
    void runSearch(
      {
        text,
        tagIds: selectedTagIds,
        startDate,
        endDate,
        limit: PAGE_SIZE,
        offset: resultsRef.current.length,
      },
      true,
    );
  }, [text, selectedTagIds, startDate, endDate, runSearch]);

  useEffect(() => {
    const sentinel = sentinelRef.current;
    if (!sentinel || !hasMore) return;
    const observer = new IntersectionObserver((records) => {
      if (records.some((record) => record.isIntersecting)) loadMore();
    });
    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [hasMore, loadMore, results]);

  const toggleTag = (tagId: string) => {
    setSelectedTagIds((current) =>
      current.includes(tagId) ? current.filter((id) => id !== tagId) : [...current, tagId],
    );
  };

  const hasCondition = text.trim() !== "" || selectedTagIds.length > 0 || startDate !== "" || endDate !== "";
  // 结果分批取，还没到底时只说读了多少，不说“找到多少”。
  const stamp =
    results && results.length > 0
      ? hasMore
        ? `已读 ${results.length} 篇`
        : `找到 ${results.length} 篇`
      : hasCondition
        ? "没有找到"
        : "全部记录";

  return (
    <section className="search-page" aria-labelledby="page-title">
      <header className="page-header">
        <div>
          <p className="eyebrow">FIND IT AGAIN</p>
          <h1 id="page-title">搜索</h1>
        </div>
        <span className="date-stamp">{stamp}</span>
      </header>

      <div className="search-controls">
        <label className="search-field">
          <SearchIcon size={15} strokeWidth={1.6} aria-hidden="true" />
          <input
            className="search-input"
            type="search"
            value={text}
            placeholder="搜索正文里的字词"
            onChange={(event) => setText(event.target.value)}
          />
        </label>

        <div className="search-dates">
          <label className="search-date">
            <span>从</span>
            <input type="date" value={startDate} onChange={(event) => setStartDate(event.target.value)} />
          </label>
          <label className="search-date">
            <span>到</span>
            <input type="date" value={endDate} onChange={(event) => setEndDate(event.target.value)} />
          </label>
        </div>

        {tags.length > 0 && (
          <div className="search-tags">
            {tags.map((tag) => (
              <button
                key={tag.id}
                type="button"
                className={`search-tag${selectedTagIds.includes(tag.id) ? " is-selected" : ""}`}
                aria-pressed={selectedTagIds.includes(tag.id)}
                onClick={() => toggleTag(tag.id)}
              >
                {tag.name}
              </button>
            ))}
          </div>
        )}
      </div>

      {results !== null && results.length === 0 ? (
        <div className="section-empty">
          <span>{hasCondition ? "换个词或去掉一些条件再试试。" : "还没有可以找回的记录。"}</span>
        </div>
      ) : (
        <ul className="search-results">
          {(results ?? []).map((entry) => (
            <li key={entry.id}>
              <button className="search-result" type="button" onClick={() => onOpenEntry(entry.entryDate, entry.id)}>
                <span className="search-result-date">{formatCalendarDate(entry.entryDate)}</span>
                <span className="search-result-body">
                  <span className="search-result-title">
                    {entry.title || entry.excerpt || "未命名的一页"}
                    <span className="search-result-time">{formatClockTime(entry.createdAtUtc)}</span>
                  </span>
                  {entry.title && <span className="search-result-excerpt">{entry.excerpt || "未命名的一页"}</span>}
                </span>
                {entry.coverAttachmentId ? (
                  <img className="search-cover" src={attachmentSource(entry.coverAttachmentId)} alt="" />
                ) : (
                  <span className="search-cover is-empty" aria-hidden="true" />
                )}
              </button>
            </li>
          ))}
        </ul>
      )}

      {hasMore && (
        <div className="timeline-more" ref={sentinelRef}>
          <span>{loading ? "正在读取…" : "继续向下滚动"}</span>
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
