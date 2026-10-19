import { useCallback, useEffect, useState } from "react";
import { RotateCcw } from "lucide-react";
import { listDeletedEntries, restoreEntry } from "../../lib/entryApi";
import { formatCalendarDate, formatClockTime } from "../../lib/localDate";
import type { DeletedEntry } from "../../types/entry";
import { errorText } from "../../lib/errorText";

export function TrashPage() {
  const [entries, setEntries] = useState<DeletedEntry[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setEntries(await listDeletedEntries());
  }, []);

  useEffect(() => {
    load().catch((cause: unknown) => setError(errorText(cause)));
  }, [load]);

  const restore = async (id: string) => {
    try {
      await restoreEntry(id);
      await load();
      setError(null);
    } catch (cause) {
      setError(errorText(cause));
    }
  };

  return (
    <section className="section-page" aria-labelledby="page-title">
      <header className="page-header">
        <div>
          <p className="eyebrow">MEMENTO TRASH</p>
          <h1 id="page-title">回收站</h1>
        </div>
      </header>

      <p className="trash-hint">移到这里的内容不会自动清理，随时可以恢复。</p>

      {entries && entries.length > 0 ? (
        <ul className="trash-list">
          {entries.map((entry) => (
            <li className="trash-item" key={entry.id}>
              <div className="trash-when">
                <span className="trash-date">{formatCalendarDate(entry.entryDate)}</span>
                <span className="trash-time">删除于 {formatClockTime(entry.deletedAtUtc)}</span>
              </div>
              <p className="trash-excerpt">{entry.title || entry.excerpt || "未命名的一页"}</p>
              <button className="link-button trash-restore" type="button" onClick={() => void restore(entry.id)}>
                <RotateCcw size={12} strokeWidth={2} aria-hidden="true" />
                恢复
              </button>
            </li>
          ))}
        </ul>
      ) : (
        <div className="section-empty">
          <span>{entries === null ? "正在读取…" : "回收站是空的"}</span>
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
