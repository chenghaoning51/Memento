use rusqlite::{Connection, OptionalExtension};

use super::Attachment;
use crate::error::AppResult;

pub fn insert(connection: &Connection, attachment: &Attachment) -> AppResult<()> {
    connection.execute(
        "INSERT INTO attachments (
            id, entry_id, relative_path, mime_type, file_size, sha256, width, height, created_at_utc
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            attachment.id,
            attachment.entry_id,
            attachment.relative_path,
            attachment.mime_type,
            attachment.file_size,
            attachment.sha256,
            attachment.width,
            attachment.height,
            attachment.created_at_utc,
        ],
    )?;
    Ok(())
}

/// 返回被删掉的相对路径，调用方据此清理磁盘文件。
pub fn delete(connection: &Connection, id: &str) -> AppResult<Option<String>> {
    let relative = connection
        .query_row(
            "SELECT relative_path FROM attachments WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .optional()?;
    connection.execute("DELETE FROM attachments WHERE id = ?1", [id])?;
    Ok(relative)
}

/// 自定义协议按 ID 取文件：先查元数据拿相对路径和 MIME，再读盘。
pub fn find_for_read(connection: &Connection, id: &str) -> AppResult<Option<(String, String)>> {
    let found = connection
        .query_row(
            "SELECT relative_path, mime_type FROM attachments WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    Ok(found)
}
