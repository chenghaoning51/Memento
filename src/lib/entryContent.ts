import type { JSONContent } from "@tiptap/core";

/** 正文格式版本：v2 起允许标题、引用、分隔线、链接和硬换行。 */
export const CONTENT_VERSION = 2;

/** v1 把段落文字放在 text 字段里，v2 用标准的 ProseMirror 行内节点。 */
type StoredNode = JSONContent & { text?: string; version?: number };

function upgradeNode(node: StoredNode): JSONContent {
  const { version: _version, ...rest } = node;
  if (rest.type === "text") return rest;
  if (typeof rest.text === "string") {
    const { text, ...block } = rest;
    return text.length > 0 ? { ...block, content: [{ type: "text", text }] } : block;
  }
  return rest.content ? { ...rest, content: rest.content.map(upgradeNode) } : rest;
}

/**
 * 读取 content_json。v1 的纯文本段落按段落边界升级为 v2，
 * 高于当前版本的正文拒绝编辑，避免降级写入丢掉新格式。
 */
export function readContent(contentJson: string): JSONContent {
  const stored = JSON.parse(contentJson) as StoredNode;
  if ((stored.version ?? 0) > CONTENT_VERSION) {
    throw new Error("这篇日记的正文格式来自更新的 Memento，请升级后再打开。");
  }
  return upgradeNode(stored);
}

/** 写入 content_json：始终带当前版本号。 */
export function writeContent(content: JSONContent): string {
  return JSON.stringify({ version: CONTENT_VERSION, ...content });
}
