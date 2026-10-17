import { useEffect, useState } from "react";
import { libraryInfo } from "../../lib/entryApi";
import type { LibraryInfo } from "../../types/entry";

export function SettingsPage() {
  const [info, setInfo] = useState<LibraryInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    libraryInfo()
      .then(setInfo)
      .catch((cause: unknown) => setError(String(cause)));
  }, []);

  return (
    <section className="section-page" aria-labelledby="page-title">
      <header className="page-header">
        <div>
          <p className="eyebrow">MEMENTO SETTINGS</p>
          <h1 id="page-title">设置</h1>
        </div>
      </header>

      <div className="settings-panel">
        <h2>数据位置</h2>
        <p className="settings-hint">日记、附件和备份都保存在这个文件夹里。</p>
        {info ? (
          <dl className="settings-facts">
            <div>
              <dt>数据目录</dt>
              <dd>{info.root}</dd>
            </div>
            <div>
              <dt>数据库文件</dt>
              <dd>{info.databasePath}</dd>
            </div>
            <div>
              <dt>数据库模式版本</dt>
              <dd>{info.schemaVersion}</dd>
            </div>
          </dl>
        ) : (
          <p className="settings-hint">{error ?? "正在读取…"}</p>
        )}
      </div>
    </section>
  );
}
