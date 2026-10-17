import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Feather, Plus } from "lucide-react";
import { attachmentSource } from "../../lib/attachmentSource";
import { createEntry, getEntry, listEntries, saveEntry } from "../../lib/entryApi";
import { readContent, writeContent } from "../../lib/entryContent";
import { formatClockTime, formatLocalDate, toLocalDateString } from "../../lib/localDate";
import type { JSONContent } from "@tiptap/core";
import type { EntrySummary, SaveStatus } from "../../types/entry";
import { EntryEditor } from "./EntryEditor";
import { useAutosave } from "./useAutosave";

type Draft = {
  id: string;
  content: JSONContent;
  contentText: string;
  coverAttachmentId: string | null;
};

const statusText: Record<SaveStatus, string> = {
  saved: "已保存",
  dirty: "编辑中",
  saving: "保存中",
  failed: "保存失败",
};

export function TodayPage() {
  const today = useMemo(() => new Date(), []);
  const entryDate = toLocalDateString(today);
  const dateLabel = formatLocalDate(today);

  const [entries, setEntries] = useState<EntrySummary[] | null>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [error, setError] = useState<string | null>(null);

  const draftRef = useRef<Draft | null>(null);
  draftRef.current = draft;

  const save = useCallback(async () => {
    const current = draftRef.current;
    if (!current) return;
    const saved = await saveEntry({
      id: current.id,
      title: "",
      contentJson: writeContent(current.content),
      contentText: current.contentText,
      coverAttachmentId: current.coverAttachmentId,
    });
    setEntries((list) =>
      list
        ? list.map((item) =>
            item.id === saved.id
              ? { ...item, excerpt: saved.excerpt, coverAttachmentId: saved.coverAttachmentId }
              : item,
          )
        : list,
    );
    setError(null);
  }, []);

  const { status, schedule, flush, retry } = useAutosave({
    entryId: draft?.id ?? null,
    save,
  });

  const refreshList = useCallback(async () => {
    setEntries(await listEntries(entryDate));
  }, [entryDate]);

  const openEntry = useCallback(
    async (id: string) => {
      if (!(await flush())) {
        setError("还有内容没有保存成功，已停留在当前日记。请先重试保存。");
        return;
      }
      try {
        const entry = await getEntry(id);
        setDraft({
          id: entry.id,
          content: readContent(entry.contentJson),
          contentText: entry.contentText,
          coverAttachmentId: entry.coverAttachmentId,
        });
        setError(null);
      } catch (cause) {
        setError(String(cause));
      }
    },
    [flush],
  );

  // 首屏只加载当天：切换日期属于 M3 的时间回看。
  useEffect(() => {
    void (async () => {
      const summaries = await listEntries(entryDate);
      setEntries(summaries);
      if (summaries.length > 0) {
        await openEntry(summaries[summaries.length - 1].id);
      }
    })();
  }, [entryDate, openEntry]);

  const startEntry = useCallback(async () => {
    if (!(await flush())) {
      setError("还有内容没有保存成功，已停留在当前日记。请先重试保存。");
      return;
    }
    try {
      const entry = await createEntry(entryDate);
      setDraft({
        id: entry.id,
        content: readContent(entry.contentJson),
        contentText: entry.contentText,
        coverAttachmentId: entry.coverAttachmentId,
      });
      setError(null);
      await refreshList();
    } catch (cause) {
      setError(String(cause));
    }
  }, [entryDate, flush, refreshList]);

  const onContentChange = (content: JSONContent, contentText: string, coverAttachmentId: string | null) => {
    setDraft((current) => (current ? { ...current, content, contentText, coverAttachmentId } : current));
    schedule();
  };

  return (
    <section className="today-page" aria-labelledby="page-title">
      <header className="page-header">
        <div>
          <p className="eyebrow">A PRIVATE JOURNAL</p>
          <h1 id="page-title">今天</h1>
        </div>
        <time className="date-stamp" dateTime={entryDate}>{dateLabel}</time>
      </header>

      <div className="journal-layout">
        <aside className="entry-list" aria-label="今天的日记">
          <div className="entry-list-head">
            <span>今天的日记</span>
            <button className="icon-button" type="button" onClick={() => void startEntry()} title="写一篇新日记">
              <Plus size={16} strokeWidth={1.8} aria-hidden="true" />
              <span className="visually-hidden">写一篇新日记</span>
            </button>
          </div>

          {entries && entries.length > 0 ? (
            <ul>
              {entries.map((entry) => (
                <li key={entry.id}>
                  <button
                    className={`entry-item${entry.coverAttachmentId ? " has-cover" : ""}${entry.id === draft?.id ? " is-active" : ""}`}
                    type="button"
                    onClick={() => void openEntry(entry.id)}
                  >
                    {entry.coverAttachmentId && (
                      <img className="entry-cover" src={attachmentSource(entry.coverAttachmentId)} alt="" />
                    )}
                    <span className="entry-time">{formatClockTime(entry.createdAtUtc)}</span>
                    <span className="entry-excerpt">{entry.excerpt || "空白日记"}</span>
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            <p className="entry-list-empty">
              {entries === null ? "正在读取…" : "今天还没有记录"}
            </p>
          )}
        </aside>

        <article className="journal-sheet">
          <div className="sheet-topline">
            <span>DAILY PAGE</span>
            <span className="sheet-index">{String(entries?.length ?? 0).padStart(2, "0")}</span>
          </div>

          {draft ? (
            <EntryEditor key={draft.id} entryId={draft.id} content={draft.content} onChange={onContentChange} />
          ) : (
            <div className="empty-page">
              <div className="empty-mark" aria-hidden="true">
                <Feather size={23} strokeWidth={1.5} />
              </div>
              <h2>今天还没有记录</h2>
              <p>平常的一天，也可以留下一点痕迹。</p>
              <button className="primary-button" type="button" onClick={() => void startEntry()}>
                写第一篇
              </button>
            </div>
          )}

          <div className="sheet-footer">
            <span className={`save-status is-${status}`} role="status">
              {statusText[status]}
            </span>
            {status === "failed" && (
              <button className="link-button" type="button" onClick={retry}>
                重试保存
              </button>
            )}
            <span className="sheet-footer-line" aria-hidden="true" />
          </div>
        </article>
      </div>

      {error && (
        <p className="page-error" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
