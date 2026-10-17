use rusqlite::{params, Connection, OptionalExtension};

use crate::entries::{Entry, EntryDraft, EntrySummary};
use crate::error::{AppError, AppResult};

const ENTRY_COLUMNS: &str = "id, entry_date, title, content_json, content_text, excerpt,
     created_at_utc, updated_at_utc, deleted_at_utc";

fn read_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<Entry> {
    Ok(Entry {
        id: row.get(0)?,
        entry_date: row.get(1)?,
        title: row.get(2)?,
        content_json: row.get(3)?,
        content_text: row.get(4)?,
        excerpt: row.get(5)?,
        created_at_utc: row.get(6)?,
        updated_at_utc: row.get(7)?,
        deleted_at_utc: row.get(8)?,
    })
}

/// 创建一篇空日记；正文写入一个最小文档结构，供 M2 的编辑器升级。
pub fn create(
    connection: &Connection,
    id: &str,
    entry_date: &str,
    content_json: &str,
    now_utc: &str,
) -> AppResult<Entry> {
    connection.execute(
        "INSERT INTO entries (id, entry_date, title, content_json, content_text, excerpt,
            created_at_utc, updated_at_utc)
         VALUES (?1, ?2, '', ?3, '', '', ?4, ?4)",
        params![id, entry_date, content_json, now_utc],
    )?;

    Ok(Entry {
        id: id.to_string(),
        entry_date: entry_date.to_string(),
        title: String::new(),
        content_json: content_json.to_string(),
        content_text: String::new(),
        excerpt: String::new(),
        created_at_utc: now_utc.to_string(),
        updated_at_utc: now_utc.to_string(),
        deleted_at_utc: None,
    })
}

pub fn load(connection: &Connection, id: &str) -> AppResult<Option<Entry>> {
    let sql = format!("SELECT {ENTRY_COLUMNS} FROM entries WHERE id = ?1");
    let entry = connection.query_row(&sql, params![id], read_entry).optional()?;
    Ok(entry)
}

/// 某一天的日记，按创建时间升序。软删除的日记不参与列表。
pub fn list_by_date(connection: &Connection, entry_date: &str) -> AppResult<Vec<EntrySummary>> {
    let mut statement = connection.prepare(
        "SELECT id, entry_date, title, excerpt, created_at_utc, updated_at_utc
         FROM entries
         WHERE entry_date = ?1 AND deleted_at_utc IS NULL
         ORDER BY created_at_utc ASC, id ASC",
    )?;

    let summaries = statement
        .query_map(params![entry_date], |row| {
            Ok(EntrySummary {
                id: row.get(0)?,
                entry_date: row.get(1)?,
                title: row.get(2)?,
                excerpt: row.get(3)?,
                created_at_utc: row.get(4)?,
                updated_at_utc: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(summaries)
}

/// 写入一次保存。返回 false 表示日记已经不存在，调用方不应把这次保存当作成功。
pub fn save(connection: &Connection, draft: &EntryDraft, excerpt: &str, now_utc: &str) -> AppResult<bool> {
    let changed = connection.execute(
        "UPDATE entries
         SET title = ?2, content_json = ?3, content_text = ?4, excerpt = ?5, updated_at_utc = ?6
         WHERE id = ?1",
        params![draft.id, draft.title, draft.content_json, draft.content_text, excerpt, now_utc],
    )?;

    Ok(changed == 1)
}

pub fn require(connection: &Connection, id: &str) -> AppResult<Entry> {
    load(connection, id)?.ok_or_else(|| AppError::MissingEntry(id.to_string()))
}

/// 摘要取正文前 80 个字符，折叠连续空白，供列表和后续搜索复用。
pub fn build_excerpt(content_text: &str) -> String {
    content_text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(80)
        .collect()
}
