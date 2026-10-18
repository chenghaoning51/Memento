import { invoke } from "@tauri-apps/api/core";
import type { Letter, LetterDraft } from "../types/letter";

export function listLetters(): Promise<Letter[]> {
  return invoke<Letter[]>("list_letters");
}

export function getLetter(id: string): Promise<Letter> {
  return invoke<Letter>("get_letter", { id });
}

export function createLetter(draft: LetterDraft): Promise<Letter> {
  return invoke<Letter>("create_letter", { draft });
}

export function updateLetter(id: string, draft: LetterDraft): Promise<Letter> {
  return invoke<Letter>("update_letter", { id, draft });
}

export function deleteLetter(id: string): Promise<void> {
  return invoke<void>("delete_letter", { id });
}

/** 打开原信：只有到期之后才能读，读到就记下打开时间。 */
export function openLetter(id: string): Promise<Letter> {
  return invoke<Letter>("open_letter", { id });
}

export function saveLetterReply(id: string, replyJson: string, replyText: string): Promise<Letter> {
  return invoke<Letter>("save_letter_reply", { id, replyJson, replyText });
}
