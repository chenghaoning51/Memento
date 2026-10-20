import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { open, ask } from "@tauri-apps/plugin-dialog";
import { Download, FolderCog, HardDriveDownload, History } from "lucide-react";
import { createBackup, inspectBackup, restoreBackup, type BackupInfo, type BackupSummary, type RestoreSummary } from "../../lib/backupApi";
import { exportEntries, type ExportSummary } from "../../lib/exportApi";
import { listTimeline, libraryInfo } from "../../lib/entryApi";
import { formatCalendarDate, formatMoment } from "../../lib/localDate";
import { relocateLibrary, resetLibraryLocation } from "../../lib/locationApi";
import type { EntrySummary, LibraryInfo } from "../../types/entry";
import { errorText } from "../../lib/errorText";

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} 字节`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export function SettingsPage() {
  const [info, setInfo] = useState<LibraryInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [exported, setExported] = useState<ExportSummary | null>(null);
  const [backedUp, setBackedUp] = useState<BackupSummary | null>(null);
  const [restored, setRestored] = useState<RestoreSummary | null>(null);
  const [busy, setBusy] = useState<"" | "export" | "backup" | "restore" | "relocate">("");
  const [relocated, setRelocated] = useState<string | null>(null);
  const [version, setVersion] = useState("");

  // 导出范围：默认全部；选过之后只导出勾中的那些。
  const [picking, setPicking] = useState(false);
  const [candidates, setCandidates] = useState<EntrySummary[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());

  useEffect(() => {
    libraryInfo()
      .then(setInfo)
      .catch((cause: unknown) => setError(errorText(cause)));
    getVersion().then(setVersion).catch(() => setVersion(""));
  }, []);

  const togglePicking = async () => {
    if (picking) {
      setPicking(false);
      return;
    }
    try {
      setCandidates(await listTimeline(200, 0));
      setSelected(new Set());
      setPicking(true);
      setError(null);
    } catch (cause) {
      setError(errorText(cause));
    }
  };

  const toggleEntry = (id: string) => {
    setSelected((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const startExport = async () => {
    const ids = picking ? [...selected] : [];
    if (picking && ids.length === 0) {
      setError("先勾选要导出的日记，或者取消选择以导出全部。");
      return;
    }
    try {
      const directory = await open({ directory: true, title: "日记导出到哪里" });
      if (!directory) return;
      setBusy("export");
      setError(null);
      setExported(await exportEntries(directory, ids));
    } catch (cause) {
      setError(errorText(cause));
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
      setError(errorText(cause));
    } finally {
      setBusy("");
    }
  };

  // 恢复前先预检并让用户确认：这份备份属于谁、有多少内容、会不会覆盖当前空间。
  const startRestore = async () => {
    try {
      const directory = await open({ directory: true, title: "选择备份文件所在的文件夹" });
      if (!directory) return;
      setBusy("restore");
      setError(null);
      const preview: BackupInfo = await inspectBackup(directory);
      const confirmed = await ask(
        `将用 ${preview.entryCount} 篇日记、${preview.letterCount} 封信（备份于 ${formatMoment(preview.createdAtUtc)}）替换现在的全部内容。恢复前会先把当前内容另存一份，确定继续吗？`,
        { title: "恢复备份", kind: "warning", okLabel: "恢复", cancelLabel: "取消" },
      );
      if (!confirmed) return;
      setRestored(await restoreBackup(directory));
      setInfo(await libraryInfo());
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy("");
    }
  };

  // 换目录会复制整个空间过去；当前连接仍指向旧位置，所以提醒重启。
  const relocate = async () => {
    try {
      const directory = await open({ directory: true, title: "日记存放到哪个文件夹" });
      if (!directory) return;
      const confirmed = await ask(
        `把现在的内容完整复制到 ${directory}，之后从那里读写。原文件夹会保留，确认继续吗？`,
        { title: "更换存放位置", kind: "warning", okLabel: "复制过去", cancelLabel: "取消" },
      );
      if (!confirmed) return;
      setBusy("relocate");
      setError(null);
      await relocateLibrary(directory);
      setRelocated(directory);
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy("");
    }
  };

  const backToDefault = async () => {
    try {
      const confirmed = await ask("下次打开应用时回到原来的存放位置。已经复制到别处的文件不会被删除，确定吗？", {
        title: "回到默认位置",
        kind: "info",
        okLabel: "回到默认",
        cancelLabel: "取消",
      });
      if (!confirmed) return;
      await resetLibraryLocation();
      setRelocated("默认位置（重新打开应用后生效）");
    } catch (cause) {
      setError(errorText(cause));
    }
  };

  const selectedCount = selected.size;

  return (
    <section className="section-page" aria-labelledby="page-title">
      <header className="page-header">
        <div>
          <h1 id="page-title">设置</h1>
        </div>
      </header>

      <div className="settings-panel">
        <h2>存放位置</h2>
        <p className="settings-hint">你的日记、图片和备份都放在这个文件夹里，全部留在这台电脑上。</p>
        {info ? (
          <dl className="settings-facts">
            <div>
              <dt>文件夹</dt>
              <dd>{info.root}</dd>
            </div>
            {version && (
              <div>
                <dt>应用版本</dt>
                <dd>{version}</dd>
              </div>
            )}
          </dl>
        ) : (
          <p className="settings-hint">{error ?? "正在读取…"}</p>
        )}
        <div className="settings-actions">
          <button className="primary-button" type="button" onClick={() => void relocate()} disabled={busy !== ""}>
            <FolderCog size={13} strokeWidth={1.9} aria-hidden="true" />
            {busy === "relocate" ? "正在复制…" : "换个文件夹存放"}
          </button>
          <button className="link-button" type="button" onClick={() => void backToDefault()} disabled={busy !== ""}>
            回到默认位置
          </button>
        </div>
        <p className="settings-hint">
          换位置会把现有内容复制过去，重新打开应用后从新文件夹读写；原来的文件夹会留着，可以确认无误后自己删掉。
        </p>
        {relocated && (
          <p className="settings-hint" role="status">
            已设置为 {relocated}。
          </p>
        )}
      </div>

      <div className="settings-panel">
        <h2>导出</h2>
        <p className="settings-hint">
          把日记导出成普通的文本文件，图片放在一起。换个软件或换个设备也能打开，不再依赖 Memento。
        </p>
        <div className="settings-actions">
          <button className="primary-button" type="button" onClick={() => void startExport()} disabled={busy !== ""}>
            <Download size={13} strokeWidth={1.9} aria-hidden="true" />
            {busy === "export" ? "正在导出…" : picking ? `导出选中的 ${selectedCount} 篇` : "导出全部日记"}
          </button>
          <button className="link-button" type="button" onClick={() => void togglePicking()} disabled={busy !== ""}>
            {picking ? "取消选择" : "只导出部分日记"}
          </button>
        </div>

        {picking && (
          <div className="export-picker">
            {candidates.length === 0 ? (
              <p className="settings-hint">还没有可以导出的日记。</p>
            ) : (
              <ul className="export-picker-list">
                {candidates.map((entry) => (
                  <li key={entry.id}>
                    <label className="export-picker-item">
                      <input
                        type="checkbox"
                        checked={selected.has(entry.id)}
                        onChange={() => toggleEntry(entry.id)}
                      />
                      <span className="export-picker-date">{formatCalendarDate(entry.entryDate)}</span>
                      <span className="export-picker-title">{entry.title || entry.excerpt || "未命名的一页"}</span>
                    </label>
                  </li>
                ))}
              </ul>
            )}
            {candidates.length > 0 && (
              <p className="settings-hint">最近 {candidates.length} 篇；更早的日记请用“导出全部日记”。</p>
            )}
          </div>
        )}

        {exported && (
          <p className="settings-hint" role="status">
            已导出 {exported.entryCount} 篇日记、{exported.imageCount} 张图片，放在 {exported.directory}
          </p>
        )}
      </div>

      <div className="settings-panel">
        <h2>备份与恢复</h2>
        <p className="settings-hint">
          备份会把你的日记、图片和它们的原始文件完整复制一份到你选的地方，换电脑或重装系统前建议先做一次。恢复会用备份里的内容替换现在看到的一切，替换前会先把当前内容另存一份。
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
        <p className="settings-hint">
          每次恢复前都会自动留一份当前内容的备份，只保留最近 5 份，更早的会自动清理。
        </p>
        {backedUp && (
          <p className="settings-hint" role="status">
            已备份 {backedUp.fileCount} 个文件（{formatBytes(backedUp.byteSize)}），放在 {backedUp.directory}
          </p>
        )}
        {restored && (
          <p className="settings-hint" role="status">
            已恢复 {restored.entryCount} 篇日记、{restored.letterCount} 封信。恢复前的备份留在 {restored.preRestoreBackup}
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
