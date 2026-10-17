import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { SaveStatus } from "../../types/entry";

/** 停止输入 800ms 后保存；连续输入时最多每 5 秒强制保存一次。 */
const IDLE_DELAY_MS = 800;
const MAX_WAIT_MS = 5000;

type AutosaveOptions = {
  entryId: string | null;
  save: () => Promise<void>;
};

export type AutosaveController = {
  status: SaveStatus;
  schedule: () => void;
  flush: () => Promise<boolean>;
  retry: () => void;
};

/**
 * 同一时刻只允许一个在途保存：保存循环把期间产生的新内容串行写下去，
 * 旧请求永远不会覆盖新内容（A03）。flush 用于切页与关闭窗口，
 * 等所有待写内容落库后才返回；失败返回 false，调用方据此取消动作（A04、A05）。
 */
export function useAutosave({ entryId, save }: AutosaveOptions): AutosaveController {
  const [status, setStatus] = useState<SaveStatus>("saved");

  const saveRef = useRef(save);
  saveRef.current = save;

  const timerRef = useRef<number | null>(null);
  const dirtySinceRef = useRef<number | null>(null);
  const pendingRef = useRef(false);
  const savingRef = useRef<Promise<boolean> | null>(null);

  const clearTimer = () => {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  };

  const drain = useCallback(async (): Promise<boolean> => {
    while (pendingRef.current) {
      pendingRef.current = false;
      setStatus("saving");
      try {
        await saveRef.current();
      } catch {
        pendingRef.current = true;
        setStatus("failed");
        return false;
      }
    }
    dirtySinceRef.current = null;
    setStatus("saved");
    return true;
  }, []);

  const runSave = useCallback((): Promise<boolean> => {
    if (savingRef.current) return savingRef.current;

    const running = drain().finally(() => {
      savingRef.current = null;
    });
    savingRef.current = running;
    return running;
  }, [drain]);

  const schedule = useCallback(() => {
    pendingRef.current = true;
    setStatus("dirty");

    const now = performance.now();
    if (dirtySinceRef.current === null) dirtySinceRef.current = now;

    clearTimer();
    const waited = now - (dirtySinceRef.current ?? now);
    const delay = Math.max(0, Math.min(IDLE_DELAY_MS, MAX_WAIT_MS - waited));
    timerRef.current = window.setTimeout(() => {
      timerRef.current = null;
      void runSave();
    }, delay);
  }, [runSave]);

  const flush = useCallback(async (): Promise<boolean> => {
    clearTimer();
    if (!pendingRef.current && !savingRef.current) return true;
    return runSave();
  }, [runSave]);

  const retry = useCallback(() => {
    clearTimer();
    pendingRef.current = true;
    void runSave();
  }, [runSave]);

  useEffect(() => {
    clearTimer();
    dirtySinceRef.current = null;
    pendingRef.current = false;
    savingRef.current = null;
    setStatus("saved");
  }, [entryId]);

  // 关闭窗口前先落盘；失败则取消关闭，保留未保存的输入（A05）。
  useEffect(() => {
    const unlisten = getCurrentWindow().onCloseRequested(async (event) => {
      if (await flush()) return;
      event.preventDefault();
    });
    return () => {
      void unlisten.then((dispose) => dispose());
    };
  }, [flush]);

  useEffect(() => clearTimer, []);

  return { status, schedule, flush, retry };
}
