export const CONTENT_VERSION = 1;

/** M1 的正文只有纯文本：按换行切成段落，写入 ADR-0003 约定的最小文档结构。 */
export function buildContentJson(text: string): string {
  const paragraphs = text.split("\n").map((line) => ({ type: "paragraph", text: line }));
  return JSON.stringify({ version: CONTENT_VERSION, type: "doc", content: paragraphs });
}
