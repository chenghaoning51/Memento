import { useEffect, useState } from "react";
import { open, ask } from "@tauri-apps/plugin-dialog";
import { Download, HardDriveDownload, History } from "lucide-react";
import { createBackup, inspectBackup, restoreBackup, type BackupInfo, type BackupSummary, type RestoreSummary } from "../../lib/backupApi";
import { exportEntries, type ExportSummary } from "../../lib/exportApi";
import { libraryInfo } from "../../lib/entryApi";
import type { LibraryInfo } from "../../types/entry";

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export function SettingsPage() {
  const [info, setInfo] = useState<LibraryInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [exported, setExported] = useState<ExportSummary | null>(null);
  const [backedUp, setBackedUp] = useState<BackupSummary | null>(null);
  const [restored, setRestored] = useState<RestoreSummary | null>(null);
  const [busy, setBusy] = useState<"" | "export" | "backup" | "restore">("");

  useEffect(() => {
    libraryInfo()
      .then(setInfo)
      .catch((cause: unknown) => setError(String(cause)));
  }, []);

  const startExport = async () => {
    try {
      const directory = await open({ directory: true, title: "选择导出的目录" });
      if (!directory) return;
      setBusy("export");
      setError(null);
      setExported(await exportEntries(directory));
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy("");
    }
  };

  const startBackup = async () => {
    try {
      const parent = await open({ directory: true, title: "选择备份存放的位置" });
      if (!parent) return;
      setBusy("backup");
      setError(null);
      setBackedUp(await createBackup(parent));
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy("");
    }
  };

  // 恢复前先预检并让用户确认：这份备份属于谁、有多少内容、会不会覆盖当前空间。
  const startRestore = async () => {
    try {
      const directory = await open({ directory: true, title: "选择要恢复的备份目录" });
      if (!directory) return;
      setBusy("restore");
      setError(null);
      const preview: BackupInfo = await inspectBackup(directory);
      const confirmed = await ask(
        `将用 ${preview.entryCount} 篇日记、${preview.letterCount} 封信（备份于 ${preview.createdAtUtc}）替换当前空间。恢复前会自动备份当前空间，确定继续吗？`,
        { title: "恢复备份", kind: "warning", okLabel: "恢复", cancelLabel: "取消" },
      );
      if (!confirmed) return;
      setRestored(await restoreBackup(directory));
      setInfo(await libraryInfo());
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy("");
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
        <button className="primary-button" type="button" onClick={() => void startExport()} disabled={busy !== ""}>
          <Download size={13} strokeWidth={1.9} aria-hidden="true" />
          {busy === "export" ? "正在导出…" : "导出为 Markdown"}
        </button>
        {exported && (
          <p className="settings-hint" role="status">
            已导出 {exported.entryCount} 篇日记、{exported.imageCount} 张图片到 {exported.directory}
          </p>
        )}
      </div>

      <div className="settings-panel">
        <h2>备份与恢复</h2>
        <p className="settings-hint">
          备份会把数据库、原图、缩略图和一份带校验值的清单复制到你选的目录。恢复会先用备份替换当前空间，替换前自动再备份一次当前空间。
        </p>
        <div className="settings-actions">
          <button className="primary-button" type="button" onClick={() => void startBackup()} disabled={busy !== ""}>
            <HardDriveDownload size={13} strokeWidth={1.9} aria-hidden="true" />
            {busy === "backup" ? "正在备份…" : "备份到文件夹"}
          </button>
          <button className="primary-button" type="button" onClick={() => void startRestore()} disabled={busy !== ""}>
            <History size={13} strokeWidth={1.9} aria-hidden="true" />
            {busy === "restore" ? "正在恢复…" : "从备份恢复"}
          </button>
        </div>
        {backedUp && (
          <p className="settings-hint" role="status">
            已备份 {backedUp.fileCount} 个文件（{formatBytes(backedUp.byteSize)}）到 {backedUp.directory}
          </p>
        )}
        {restored && (
          <p className="settings-hint" role="status">
            已恢复 {restored.entryCount} 篇日记、{restored.letterCount} 封信；恢复前的空间备份在 {restored.preRestoreBackup}
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
