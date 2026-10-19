import { invoke } from "@tauri-apps/api/core";

/** 把整个空间复制到新目录，并记下新位置；重启后生效。 */
export function relocateLibrary(directory: string): Promise<void> {
  return invoke("relocate_library", { directory });
}

/** 回到默认数据目录；重启后生效。 */
export function resetLibraryLocation(): Promise<void> {
  return invoke("reset_library_location");
}
