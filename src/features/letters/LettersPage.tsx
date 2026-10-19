import { useCallback, useEffect, useState } from "react";
import { Lock, MailOpen, Trash2 } from "lucide-react";
import { buildLetterContent } from "../../lib/letterContent";
import {
  createLetter,
  deleteLetter,
  getLetter,
  listLetters,
  openLetter,
  saveLetterReply,
  updateLetter,
} from "../../lib/letterApi";
import { formatCalendarDate, toLocalDateString } from "../../lib/localDate";
import { currentTimeZone, daysUntil, formatMomentInZone, unlockAtUtc, unlockLocalDate } from "../../lib/unlockTime";
import type { Letter } from "../../types/letter";
import { errorText } from "../../lib/errorText";

const DAY_MS = 86_400_000;

/** 列表里的短说明：锁着说还有多久，到期说可以读了。 */
function describe(letter: Letter, now: Date): string {
  if (letter.status === "locked") {
    const days = daysUntil(letter.unlockAtUtc, letter.unlockTimezone, now);
    return days > 0 ? `还有 ${days} 天开启` : "今天开启";
  }
  return letter.status === "opened" ? "读过，可以补回应" : "可以打开";
}

function tomorrow(): string {
  return toLocalDateString(new Date(Date.now() + DAY_MS));
}

export function LettersPage() {
  const [letters, setLetters] = useState<Letter[] | null>(null);
  const [current, setCurrent] = useState<Letter | null>(null);
  const [reading, setReading] = useState(false);
  const [title, setTitle] = useState("");
  const [text, setText] = useState("");
  const [unlockDate, setUnlockDate] = useState(tomorrow);
  const [reply, setReply] = useState("");
  const [error, setError] = useState<string | null>(null);
  // 列表上的「还有几天开启」按打开页面时的钟算，不跟着时间自己跳。
  const [now, setNow] = useState(() => new Date());

  const load = useCallback(async () => {
    setLetters(await listLetters());
  }, []);

  useEffect(() => {
    setNow(new Date());
    load().catch((cause: unknown) => setError(errorText(cause)));
  }, [load]);

  const startWriting = () => {
    setCurrent(null);
    setReading(false);
    setTitle("");
    setText("");
    setUnlockDate(tomorrow());
    setError(null);
  };

  // 还没到期的信可以接着改：开启日期按写信时的时区读回来，不会因为换了时区挪一天。
  // 列表是进页面时读的，点开前再问一次，刚到期的那封就直接按阅读打开。
  const startEditing = async (letter: Letter) => {
    try {
      const fresh = await getLetter(letter.id);
      if (fresh.status !== "locked") {
        await startReading(fresh);
        return;
      }
      setCurrent(fresh);
      setReading(false);
      setTitle(fresh.title);
      setText(fresh.contentText);
      setUnlockDate(unlockLocalDate(fresh.unlockAtUtc, fresh.unlockTimezone));
      setError(null);
    } catch (cause) {
      setError(errorText(cause));
    }
  };

  // 到期后的信只读：第一次打开记下打开时间，之后再回来还能看到原信与回应。
  const startReading = async (letter: Letter) => {
    try {
      const opened = letter.status === "opened" ? await getLetter(letter.id) : await openLetter(letter.id);
      setCurrent(opened);
      setReading(true);
      setReply(opened.replyText ?? "");
      setError(null);
      await load();
    } catch (cause) {
      setError(errorText(cause));
    }
  };

  // 日期没改就沿用原来的开启时刻：换个时区编辑不该让约定好的时刻漂移。
  // 改了日期则按当前时区重新换算，时区标签跟着换成当前时区，时刻与标签始终一致。
  const storedMoment = current !== null && unlockDate === unlockLocalDate(current.unlockAtUtc, current.unlockTimezone);
  const unlockMoment = storedMoment && current ? current.unlockAtUtc : unlockAtUtc(unlockDate);
  const unlockZone = storedMoment && current ? current.unlockTimezone : currentTimeZone();

  const save = async () => {
    if (text.trim().length === 0) {
      setError("先写一点内容再封存。");
      return;
    }
    const { contentJson, contentText } = buildLetterContent(text);
    const draft = {
      title: title.trim(),
      contentJson,
      contentText,
      unlockAtUtc: unlockMoment,
      unlockTimezone: unlockZone,
    };
    try {
      if (current) await updateLetter(current.id, draft);
      else await createLetter(draft);
      startWriting();
      await load();
    } catch (cause) {
      setError(errorText(cause));
    }
  };

  const saveReply = async () => {
    if (!current) return;
    const { contentJson, contentText } = buildLetterContent(reply);
    try {
      setCurrent(await saveLetterReply(current.id, contentJson, contentText));
      setError(null);
      await load();
    } catch (cause) {
      setError(errorText(cause));
    }
  };

  const remove = async (id: string) => {
    try {
      await deleteLetter(id);
      startWriting();
      await load();
    } catch (cause) {
      setError(errorText(cause));
    }
  };

  return (
    <section className="section-page" aria-labelledby="page-title">
      <header className="page-header">
        <div>
          <p className="eyebrow">MEMENTO LETTERS</p>
          <h1 id="page-title">未来信箱</h1>
        </div>
        <button className="primary-button letter-new" type="button" onClick={startWriting}>
          写一封信
        </button>
      </header>

      <p className="section-hint">
        写给以后的自己。到开启那天之前，正文在应用里保持关闭；这不是加密，只是给自己的一个约定。
      </p>

      <div className="letters-layout">
        <ul className="letter-list">
          {letters && letters.length > 0 ? (
            letters.map((letter) => (
              <li key={letter.id}>
                <button
                  className={"letter-item" + (current?.id === letter.id ? " is-active" : "")}
                  type="button"
                  onClick={() => (letter.status === "locked" ? void startEditing(letter) : void startReading(letter))}
                >
                  <span className={"letter-mark" + (letter.status === "locked" ? "" : " is-open")}>
                    {letter.status === "locked" ? (
                      <Lock size={12} strokeWidth={1.9} aria-hidden="true" />
                    ) : (
                      <MailOpen size={12} strokeWidth={1.9} aria-hidden="true" />
                    )}
                  </span>
                  <span className="letter-heading">{letter.title || "没有标题的信"}</span>
                  <span className="letter-note">{describe(letter, now)}</span>
                  <span className="letter-when">
                    {formatCalendarDate(unlockLocalDate(letter.unlockAtUtc, letter.unlockTimezone))}
                  </span>
                </button>
              </li>
            ))
          ) : (
            <li className="section-empty">
              <span>{letters === null ? "正在读取…" : "还没有写给未来的信"}</span>
            </li>
          )}
        </ul>

        <div className="letter-sheet">
          {reading && current ? (
            <>
              <div className="sheet-topline">
                <span className="sheet-index">原信</span>
                <span>{formatMomentInZone(current.unlockAtUtc, current.unlockTimezone)} 开启</span>
              </div>
              <h2 className="letter-title">{current.title || "没有标题的信"}</h2>
              <p className="letter-body">{current.contentText}</p>
              <div className="letter-reply">
                <label className="letter-reply-label" htmlFor="letter-reply">
                  写下此刻的回应
                </label>
                <textarea
                  id="letter-reply"
                  className="letter-reply-input"
                  value={reply}
                  onChange={(event) => setReply(event.target.value)}
                  placeholder="现在的你，想对写信的那天说点什么？"
                />
                <button className="link-button" type="button" onClick={() => void saveReply()}>
                  保存回应
                </button>
              </div>
              <div className="sheet-footer">
                <button className="link-button" type="button" onClick={startWriting}>
                  写新的一封
                </button>
                <span className="sheet-footer-line" aria-hidden="true" />
              </div>
            </>
          ) : (
            <>
              <div className="sheet-topline">
                <span className="sheet-index">{current ? "编辑" : "新信"}</span>
                <span>{formatMomentInZone(unlockMoment, unlockZone)} 开启</span>
              </div>
              <input
                className="letter-title-input"
                value={title}
                onChange={(event) => setTitle(event.target.value)}
                placeholder="给这封信起个标题"
                aria-label="信件标题"
              />
              <textarea
                className="letter-input"
                value={text}
                onChange={(event) => setText(event.target.value)}
                placeholder="写下此刻想留给未来的话。"
                aria-label="信件正文"
              />
              <div className="letter-unlock">
                <label className="letter-unlock-label" htmlFor="unlock-date">
                  开启日期
                </label>
                <input
                  id="unlock-date"
                  className="letter-date"
                  type="date"
                  value={unlockDate}
                  min={toLocalDateString(new Date())}
                  onChange={(event) => setUnlockDate(event.target.value)}
                />
                <span className="letter-unlock-hint">当天 00:00 之后可以打开</span>
              </div>
              <div className="sheet-footer">
                <button className="link-button letter-save" type="button" onClick={() => void save()}>
                  {current ? "保存修改" : "封存这封信"}
                </button>
                {current && (
                  <button className="link-button" type="button" onClick={() => void remove(current.id)}>
                    <Trash2 size={11} strokeWidth={2} aria-hidden="true" />
                    删除
                  </button>
                )}
                <span className="sheet-footer-line" aria-hidden="true" />
              </div>
            </>
          )}
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
