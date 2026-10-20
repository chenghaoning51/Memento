import { useCallback, useEffect, useState } from "react";
import { listLetters } from "../lib/letterApi";

/** 到期待读的信：已经过了开启时刻，但用户还没打开过。 */
export function useUnreadLetters(): { count: number; refresh: () => void } {
  const [count, setCount] = useState(0);

  const refresh = useCallback(() => {
    listLetters()
      .then((letters) => {
        setCount(letters.filter((letter) => letter.status === "unlocked").length);
      })
      // 角标只是提醒，读不到时静默——不要为它弹错误打断写作。
      .catch(() => setCount(0));
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { count, refresh };
}
