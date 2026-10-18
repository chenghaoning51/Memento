export type LetterStatus = "locked" | "unlocked" | "opened";

export type Letter = {
  id: string;
  title: string;
  contentJson: string;
  contentText: string;
  unlockAtUtc: string;
  unlockTimezone: string;
  status: LetterStatus;
  replyJson: string | null;
  replyText: string | null;
  openedAtUtc: string | null;
  createdAtUtc: string;
  updatedAtUtc: string;
};

export type LetterDraft = {
  title: string;
  contentJson: string;
  contentText: string;
  unlockAtUtc: string;
  unlockTimezone: string;
};
