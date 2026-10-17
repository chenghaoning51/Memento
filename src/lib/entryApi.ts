import { invoke } from "@tauri-apps/api/core";
import type {
  Attachment,
  DeletedEntry,
  Entry,
  EntryDateSummary,
  EntryDraft,
  EntrySummary,
  LibraryInfo,
  Tag,
} from "../types/entry";

export function libraryInfo(): Promise<LibraryInfo> {
  return invoke<LibraryInfo>("library_info");
}

export function listEntries(entryDate: string): Promise<EntrySummary[]> {
  return invoke<EntrySummary[]>("list_entries", { entryDate });
}

/** 月历用：startDate 到 endDate（含）之间每个有记录的本地日期。 */
export function listEntryDates(startDate: string, endDate: string): Promise<EntryDateSummary[]> {
  return invoke<EntryDateSummary[]>("list_entry_dates", { startDate, endDate });
}

/** 时间线用：按日期倒序分批取日记。 */
export function listTimeline(limit: number, offset: number): Promise<EntrySummary[]> {
  return invoke<EntrySummary[]>("list_timeline", { limit, offset });
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

export function deleteEntry(id: string): Promise<void> {
  return invoke<void>("delete_entry", { id });
}

export function listDeletedEntries(): Promise<DeletedEntry[]> {
  return invoke<DeletedEntry[]>("list_deleted_entries");
}

export function restoreEntry(id: string): Promise<Entry> {
  return invoke<Entry>("restore_entry", { id });
}

export function listEntryTags(entryId: string): Promise<Tag[]> {
  return invoke<Tag[]>("list_entry_tags", { entryId });
}

export function addEntryTag(entryId: string, name: string): Promise<Tag[]> {
  return invoke<Tag[]>("add_entry_tag", { entryId, name });
}

export function removeEntryTag(entryId: string, tagId: string): Promise<Tag[]> {
  return invoke<Tag[]>("remove_entry_tag", { entryId, tagId });
}
