import { invoke } from "@tauri-apps/api/core";
import type { Attachment, Entry, EntryDraft, EntrySummary, LibraryInfo } from "../types/entry";

export function libraryInfo(): Promise<LibraryInfo> {
  return invoke<LibraryInfo>("library_info");
}

export function listEntries(entryDate: string): Promise<EntrySummary[]> {
  return invoke<EntrySummary[]>("list_entries", { entryDate });
}

export function getEntry(id: string): Promise<Entry> {
  return invoke<Entry>("get_entry", { id });
}

export function createEntry(entryDate: string): Promise<Entry> {
  return invoke<Entry>("create_entry", { entryDate });
}

export function saveEntry(draft: EntryDraft): Promise<Entry> {
  return invoke<Entry>("save_entry", { draft });
}

export function importAttachment(entryId: string, bytes: Uint8Array): Promise<Attachment> {
  return invoke<Attachment>("import_attachment", { entryId, bytes: Array.from(bytes) });
}

export function deleteAttachment(id: string): Promise<void> {
  return invoke<void>("delete_attachment", { id });
}
