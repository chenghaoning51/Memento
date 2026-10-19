/**
 * Tauri 命令把 Rust 的 AppError 序列化成字符串，前端拿到的却是 Error 对象；
 * 直接 String() 会得到 "Error: 文件操作失败：..."。这里只保留应用写给用户的那句话。
 */
export function errorText(cause: unknown): string {
  if (typeof cause === "string") return cause;
  if (cause instanceof Error) return cause.message;
  return String(cause);
}
