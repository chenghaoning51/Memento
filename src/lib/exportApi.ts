import { invoke } from "@tauri-apps/api/core";

export type ExportSummary = {
  directory: string;
  entryCount: number;
  imageCount: number;
};

/** 把全部日记导出成 Markdown 与图片，写进用户选定的目录。 */
export function exportEntries(directory: string): Promise<ExportSummary> {
  return invoke("export_entries", { directory });
}
