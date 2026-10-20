import { useCallback, useEffect, useRef, useState } from "react";
import { Feather, Plus } from "lucide-react";
import { attachmentSource } from "../../lib/attachmentSource";
import { createEntry, deleteAttachment, deleteEntry, getEntry, listEntries, saveEntry } from "../../lib/entryApi";
import { readContent, writeContent } from "../../lib/entryContent";
import { formatCalendarDate, formatClockTime, formatLocalDate, parseLocalDate, toLocalDateString } from "../../lib/localDate";
import type { JSONContent } from "@tiptap/core";
import type { EntrySummary, SaveStatus } from "../../types/entry";
import { EntryEditor } from "./EntryEditor";
import { TagBar } from "./TagBar";
import { useAutosave } from "./useAutosave";
import { errorText } from "../../lib/errorText";

type Draft = {
  id: string;
  title: string;
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

type JournalPageProps = {
  /** 本地日历日期（YYYY-MM-DD）；缺省为今天。 */
  entryDate?: string;
  /** 从时间线点进来时，直接打开指定的那一篇。 */
  entryId?: string;
  /** 从时间线或月历进来时显示的返回入口。 */
  back?: { label: string; onClick: () => void };
  /** 点标签时跳到已带上该标签的搜索页。 */
  onOpenTag?: (tagId: string) => void;
  registerLeaveGuard: (guard: (() => Promise<boolean>) | null) => void;
};

/** 某一天的日记页：今天页与时间线点进来的日期共用同一套编辑、保存和切页规则。 */
export function JournalPage({
  entryDate: requestedDate,
  entryId: requestedEntryId,
  back,
  onOpenTag,
  registerLeaveGuard,
}: JournalPageProps) {
  const entryDate = requestedDate ?? toLocalDateString(new Date());
  const isToday = entryDate === toLocalDateString(new Date());
  const heading = isToday ? "今天" : formatCalendarDate(entryDate);
  const listLabel = isToday ? "今天的日记" : "这一天的日记";

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
      title: current.title,
      contentJson: writeContent(current.content),
      contentText: current.contentText,
      coverAttachmentId: current.coverAttachmentId,
    });
    setEntries((list) =>
      list
        ? list.map((item) =>
            item.id === saved.id
              ? { ...item, title: saved.title, excerpt: saved.excerpt, coverAttachmentId: saved.coverAttachmentId }
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
    const summaries = await listEntries(entryDate);
    setEntries(summaries);
    return summaries;
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
          title: entry.title,
          content: readContent(entry.contentJson),
          contentText: entry.contentText,
          coverAttachmentId: entry.coverAttachmentId,
        });
        setError(null);
      } catch (cause) {
        setError(errorText(cause));
      }
    },
    [flush],
  );

  // 换到另一天时先清空旧草稿，再按新日期重载列表并打开当天最后一篇。
  useEffect(() => {
    setDraft(null);
    void (async () => {
      const summaries = await listEntries(entryDate);
      setEntries(summaries);
      const target = summaries.some((item) => item.id === requestedEntryId)
        ? requestedEntryId
        : summaries[summaries.length - 1]?.id;
      if (target) await openEntry(target);
    })();
  }, [entryDate, requestedEntryId, openEntry]);

  const startEntry = useCallback(async () => {
    if (!(await flush())) {
      setError("还有内容没有保存成功，已停留在当前日记。请先重试保存。");
      return;
    }
    try {
      const entry = await createEntry(entryDate);
      setDraft({
        id: entry.id,
        title: entry.title,
        content: readContent(entry.contentJson),
        contentText: entry.contentText,
        coverAttachmentId: entry.coverAttachmentId,
      });
      setError(null);
      await refreshList();
    } catch (cause) {
      setError(errorText(cause));
    }
  }, [entryDate, flush, refreshList]);

  // 软删除：先落盘再移走，附件与标签关系都不动（恢复时原样回来）。
  const moveToTrash = useCallback(async () => {
    if (!draft || !(await flush())) {
      setError("还有内容没有保存成功，已停留在当前日记。请先重试保存。");
      return;
    }
    try {
      await deleteEntry(draft.id);
      setDraft(null);
      const remaining = await refreshList();
      setError(null);
      if (remaining.length > 0) {
        await openEntry(remaining[remaining.length - 1].id);
      }
    } catch (cause) {
      setError(errorText(cause));
    }
  }, [draft, flush, openEntry, refreshList]);

  // 侧栏切页同样先落盘：失败就留在当前日记，输入不丢（A04）。
  useEffect(() => {
    registerLeaveGuard(async () => {
      if (await flush()) return true;
      setError("还有内容没有保存成功，已停留在当前日记。请先重试保存。");
      return false;
    });
    return () => registerLeaveGuard(null);
  }, [flush, registerLeaveGuard]);

  const onTitleChange = (title: string) => {
    setDraft((current) => (current ? { ...current, title } : current));
    schedule();
  };

  const onContentChange = (content: JSONContent, contentText: string, coverAttachmentId: string | null) => {
    setDraft((current) => (current ? { ...current, content, contentText, coverAttachmentId } : current));
    schedule();
  };

  // 图片已从正文里摘掉，先把新正文落盘，再删磁盘文件：
  // 反过来的话，保存失败就会留下一条指向已删除文件的引用。
  const onRemoveAttachment = useCallback(
    async (
      attachmentId: string,
      content: JSONContent,
      contentText: string,
      coverAttachmentId: string | null,
    ) => {
      const current = draftRef.current;
      if (!current) return;
      const next = { ...current, content, contentText, coverAttachmentId };
      setDraft(next);
      // 直接写这一份，不等自动保存的定时器，也不用读可能还没刷新的 ref。
      await saveEntry({
        id: next.id,
        title: next.title,
        contentJson: writeContent(next.content),
        contentText: next.contentText,
        coverAttachmentId: next.coverAttachmentId,
      });
      await deleteAttachment(attachmentId);
    },
    [],
  );

  return (
    <section className="journal-page" aria-labelledby="page-title">
      <header className="page-header">
        <div>
          {back && (
            <button className="page-back" type="button" onClick={back.onClick}>
              {back.label}
            </button>
          )}
          <h1 id="page-title">{heading}</h1>
        </div>
        <time className="date-stamp" dateTime={entryDate}>{formatLocalDate(parseLocalDate(entryDate))}</time>
      </header>

      <div className="journal-layout">
        <aside className="entry-list" aria-label={listLabel}>
          <div className="entry-list-head">
            <span>{listLabel}</span>
            <button className="icon-button" type="button" onClick={() => void startEntry()} title="写一篇">
              <Plus size={16} strokeWidth={1.8} aria-hidden="true" />
              <span className="visually-hidden">写一篇</span>
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
                    <span className="entry-excerpt">{entry.title || entry.excerpt || "未命名的一页"}</span>
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            <p className="entry-list-empty">
              {entries === null ? "正在读取…" : isToday ? "今天还没有记录" : "这一天没有记录"}
            </p>
          )}
        </aside>

        <article className="journal-sheet">
          {draft ? (
            <>
              <input
                className="entry-title"
                type="text"
                value={draft.title}
                placeholder="无题"
                aria-label="日记标题"
                onChange={(event) => onTitleChange(event.target.value)}
              />
              <EntryEditor
                key={`editor-${draft.id}`}
                entryId={draft.id}
                content={draft.content}
                onChange={onContentChange}
                onRemoveAttachment={onRemoveAttachment}
              />
            </>
          ) : (
            <div className="empty-page">
              <div className="empty-mark" aria-hidden="true">
                <Feather size={23} strokeWidth={1.5} />
              </div>
              <h2>{isToday ? "今天还没有记录" : "这一天还没有记录"}</h2>
              <p>{isToday ? "平常的一天，也可以留下一点痕迹。" : "补写这一天，也来得及。"}</p>
              <button className="primary-button" type="button" onClick={() => void startEntry()}>
                {isToday ? "写今天的第一篇" : "写这一天的第一篇"}
              </button>
            </div>
          )}

          {draft && <TagBar key={`tags-${draft.id}`} entryId={draft.id} onOpenTag={onOpenTag} />}

          <div className="sheet-footer">
            <span className={`save-status is-${status}`} role="status">
              {statusText[status]}
            </span>
            {status === "failed" && (
              <button className="link-button" type="button" onClick={retry}>
                重试保存
              </button>
            )}
            {draft && (
              <button className="link-button sheet-trash" type="button" onClick={() => void moveToTrash()}>
                移到回收站
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
