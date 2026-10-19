import { invoke } from "@tauri-apps/api/core";

export type ExportSummary = {
  directory: string;
  entryCount: number;
  imageCount: number;
};

/** 把日记导出成 Markdown 与图片，写进用户选定的目录。传空数组表示全部。 */
export function exportEntries(directory: string, entryIds: string[] = []): Promise<ExportSummary> {
  return invoke("export_entries", { directory, entryIds });
}
