import { invoke } from "@tauri-apps/api/core";

export type BackupSummary = {
  directory: string;
  fileCount: number;
  byteSize: number;
};

export type BackupInfo = {
  directory: string;
  appVersion: string;
  schemaVersion: number;
  createdAtUtc: string;
  entryCount: number;
  letterCount: number;
  fileCount: number;
  byteSize: number;
};

export type RestoreSummary = {
  entryCount: number;
  letterCount: number;
  preRestoreBackup: string;
};

/** 备份整个空间到选定目录下的带时间戳子目录。 */
export function createBackup(parent: string): Promise<BackupSummary> {
  return invoke("create_backup", { parent });
}

/** 恢复前预检：manifest 版本、文件清单与校验值。 */
export function inspectBackup(directory: string): Promise<BackupInfo> {
  return invoke("inspect_backup", { directory });
}

/** 用备份替换当前空间；替换前会自动备份当前空间。 */
export function restoreBackup(directory: string): Promise<RestoreSummary> {
  return invoke("restore_backup", { directory });
}
