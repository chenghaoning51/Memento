use rusqlite::Connection;

use super::ExportEntry;
use crate::error::AppResult;

/// 导出用的日记：按日期正序，同一天内按写入顺序，回收站里的不导出。
pub fn list_for_export(connection: &Connection) -> AppResult<Vec<ExportEntry>> {
    let mut statement = connection.prepare(
        "SELECT entry_date, title, content_json, excerpt
         FROM entries
         WHERE deleted_at_utc IS NULL
         ORDER BY entry_date ASC, created_at_utc ASC, id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(ExportEntry {
            entry_date: row.get(0)?,
            title: row.get(1)?,
            content_json: row.get(2)?,
            excerpt: row.get(3)?,
        })
    })?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

/// 附件 ID 到相对路径的映射：导出时按正文里引用的 ID 找图片。
pub fn attachment_paths(connection: &Connection) -> AppResult<std::collections::HashMap<String, String>> {
    let mut statement = connection.prepare("SELECT id, relative_path FROM attachments")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut paths = std::collections::HashMap::new();
    for row in rows {
        let (id, relative) = row?;
        paths.insert(id, relative);
    }
    Ok(paths)
}
