export type Entry = {
  id: string;
  entryDate: string;
  title: string;
  contentJson: string;
  contentText: string;
  excerpt: string;
  createdAtUtc: string;
  updatedAtUtc: string;
  deletedAtUtc: string | null;
};

export type EntrySummary = {
  id: string;
  entryDate: string;
  title: string;
  excerpt: string;
  createdAtUtc: string;
  updatedAtUtc: string;
};

export type EntryDraft = {
  id: string;
  title: string;
  contentJson: string;
  contentText: string;
};

export type SaveStatus = "saved" | "dirty" | "saving" | "failed";

export type LibraryInfo = {
  root: string;
  databasePath: string;
  schemaVersion: number;
};
