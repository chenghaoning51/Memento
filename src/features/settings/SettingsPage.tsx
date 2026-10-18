import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Download } from "lucide-react";
import { exportEntries, type ExportSummary } from "../../lib/exportApi";
import { libraryInfo } from "../../lib/entryApi";
import type { LibraryInfo } from "../../types/entry";

export function SettingsPage() {
  const [info, setInfo] = useState<LibraryInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [summary, setSummary] = useState<ExportSummary | null>(null);
  const [exporting, setExporting] = useState(false);

  useEffect(() => {
    libraryInfo()
      .then(setInfo)
      .catch((cause: unknown) => setError(String(cause)));
  }, []);

  // 导出目录由用户在系统对话框里选，取消时不报错也不留半成品。
  const startExport = async () => {
    try {
      const directory = await open({ directory: true, title: "选择导出的目录" });
      if (!directory) return;
      setExporting(true);
      setError(null);
      setSummary(await exportEntries(directory));
    } catch (cause) {
      setError(String(cause));
    } finally {
      setExporting(false);
    }
  };

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

      <div className="settings-panel">
        <h2>导出</h2>
        <p className="settings-hint">
          把全部日记写成 Markdown 文件，图片复制到同一目录的 images/ 文件夹里。导出后不依赖 Memento 也能打开。
        </p>
        <button className="primary-button" type="button" onClick={() => void startExport()} disabled={exporting}>
          <Download size={13} strokeWidth={1.9} aria-hidden="true" />
          {exporting ? "正在导出…" : "导出为 Markdown"}
        </button>
        {summary && (
          <p className="settings-hint" role="status">
            已导出 {summary.entryCount} 篇日记、{summary.imageCount} 张图片到 {summary.directory}
          </p>
        )}
      </div>

      {error && (
        <p className="page-error" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
