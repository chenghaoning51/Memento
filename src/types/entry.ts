export type Entry = {
  id: string;
  entryDate: string;
  title: string;
  contentJson: string;
  contentText: string;
  excerpt: string;
  coverAttachmentId: string | null;
  createdAtUtc: string;
  updatedAtUtc: string;
  deletedAtUtc: string | null;
};

export type EntrySummary = {
  id: string;
  entryDate: string;
  title: string;
  excerpt: string;
  coverAttachmentId: string | null;
  createdAtUtc: string;
  updatedAtUtc: string;
};

export type EntryDraft = {
  id: string;
  title: string;
  contentJson: string;
  contentText: string;
  coverAttachmentId: string | null;
};

export type Attachment = {
  id: string;
  entryId: string;
  relativePath: string;
  mimeType: string;
  fileSize: number;
  sha256: string;
  width: number;
  height: number;
  createdAtUtc: string;
};

export type Tag = {
  id: string;
  name: string;
};

export type DeletedEntry = {
  id: string;
  entryDate: string;
  excerpt: string;
  coverAttachmentId: string | null;
  createdAtUtc: string;
  deletedAtUtc: string;
};

export type SaveStatus = "saved" | "dirty" | "saving" | "failed";

export type LibraryInfo = {
  root: string;
  databasePath: string;
  schemaVersion: number;
};
