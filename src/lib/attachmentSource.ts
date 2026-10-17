import { convertFileSrc } from "@tauri-apps/api/core";

/**
 * 原图通过 attachment 自定义协议按附件 ID 读取，正文和列表里只记 ID，不存路径也不存 Base64。
 * 协议 URL 在各平台形态不同（Windows 是 http://attachment.localhost/<id>），交给 convertFileSrc 拼接。
 */
export function attachmentSource(id: string): string {
  return convertFileSrc(id, "attachment");
}
