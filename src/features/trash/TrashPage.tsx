import { useCallback, useEffect, useState } from "react";
import { RotateCcw, Trash2 } from "lucide-react";
import { ask } from "@tauri-apps/plugin-dialog";
import { listDeletedEntries, purgeEntry, purgeTrash, restoreEntry } from "../../lib/entryApi";
import { formatCalendarDate, formatClockTime } from "../../lib/localDate";
import type { DeletedEntry } from "../../types/entry";
import { errorText } from "../../lib/errorText";

export function TrashPage() {
  const [entries, setEntries] = useState<DeletedEntry[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

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

  // 永久删除不可撤销，先确认再动手。
  const purgeOne = async (entry: DeletedEntry) => {
    const label = entry.title || entry.excerpt || "未命名的一页";
    const confirmed = await ask(`「${label}」将被永久删除，正文和图片都无法恢复。`, {
      title: "永久删除",
      kind: "warning",
      okLabel: "永久删除",
      cancelLabel: "取消",
    });
    if (!confirmed) return;
    setBusy(true);
    try {
      await purgeEntry(entry.id);
      await load();
      setError(null);
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };

  const emptyTrash = async () => {
    const count = entries?.length ?? 0;
    const confirmed = await ask(`回收站里的 ${count} 篇日记将被永久删除，正文和图片都无法恢复。`, {
      title: "清空回收站",
      kind: "warning",
      okLabel: "清空",
      cancelLabel: "取消",
    });
    if (!confirmed) return;
    setBusy(true);
    try {
      await purgeTrash();
      await load();
      setError(null);
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };

  const hasEntries = !!entries && entries.length > 0;

  return (
    <section className="section-page" aria-labelledby="page-title">
      <header className="page-header">
        <div>
          <h1 id="page-title">回收站</h1>
        </div>
        {hasEntries && (
          <button className="link-button" type="button" onClick={() => void emptyTrash()} disabled={busy}>
            <Trash2 size={12} strokeWidth={2} aria-hidden="true" />
            清空回收站
          </button>
        )}
      </header>

      <p className="trash-hint">移到这里的内容不会自动清理，随时可以恢复；永久删除后无法找回。</p>

      {hasEntries ? (
        <ul className="trash-list">
          {entries.map((entry) => (
            <li className="trash-item" key={entry.id}>
              <div className="trash-when">
                <span className="trash-date">{formatCalendarDate(entry.entryDate)}</span>
                <span className="trash-time">删除于 {formatClockTime(entry.deletedAtUtc)}</span>
              </div>
              <p className="trash-excerpt">{entry.title || entry.excerpt || "未命名的一页"}</p>
              <div className="trash-actions">
                <button className="link-button" type="button" onClick={() => void restore(entry.id)} disabled={busy}>
                  <RotateCcw size={12} strokeWidth={2} aria-hidden="true" />
                  恢复
                </button>
                <button className="link-button is-danger" type="button" onClick={() => void purgeOne(entry)} disabled={busy}>
                  <Trash2 size={12} strokeWidth={2} aria-hidden="true" />
                  永久删除
                </button>
              </div>
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
