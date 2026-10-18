import type { JSONContent } from "@tiptap/core";
import { writeContent } from "./entryContent";

/**
 * 信件正文先按纯文本段落保存，复用日记的 v2 正文格式，
 * 以后换成富文本编辑器时不用再迁移一次。
 */
export function buildLetterContent(text: string): { contentJson: string; contentText: string } {
  const lines = text.replace(/\r\n/g, "\n").split("\n");
  const content: JSONContent = {
    type: "doc",
    content: lines.map((line) =>
      line.length > 0 ? { type: "paragraph", content: [{ type: "text", text: line }] } : { type: "paragraph" },
    ),
  };
  return { contentJson: writeContent(content), contentText: text.trim() };
}
