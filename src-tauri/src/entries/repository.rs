use rusqlite::{Connection, OptionalExtension};

use super::{DeletedEntry, Entry, EntryDraft, EntrySummary};
use crate::error::{AppError, AppResult};

const ENTRY_COLUMNS: &str = "id, entry_date, title, content_json, content_text, excerpt, cover_attachment_id,
     created_at_utc, updated_at_utc, deleted_at_utc";

fn read_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<Entry> {
    Ok(Entry {
        id: row.get(0)?,
        entry_date: row.get(1)?,
        title: row.get(2)?,
        content_json: row.get(3)?,
        content_text: row.get(4)?,
        excerpt: row.get(5)?,
        cover_attachment_id: row.get(6)?,
        created_at_utc: row.get(7)?,
        updated_at_utc: row.get(8)?,
        deleted_at_utc: row.get(9)?,
    })
}

pub fn create(
    connection: &Connection,
    id: &str,
    entry_date: &str,
    content_json: &str,
    now_utc: &str,
) -> AppResult<Entry> {
    connection.execute(
        "INSERT INTO entries (id, entry_date, title, content_json, content_text, excerpt, created_at_utc, updated_at_utc)
         VALUES (?1, ?2, '', ?3, '', '', ?4, ?4)",
        rusqlite::params![id, entry_date, content_json, now_utc],
    )?;

    Ok(Entry {
        id: id.to_string(),
        entry_date: entry_date.to_string(),
        title: String::new(),
        content_json: content_json.to_string(),
        content_text: String::new(),
        excerpt: String::new(),
        cover_attachment_id: None,
        created_at_utc: now_utc.to_string(),
        updated_at_utc: now_utc.to_string(),
        deleted_at_utc: None,
    })
}

pub fn load(connection: &Connection, id: &str) -> AppResult<Option<Entry>> {
    let entry = connection
        .query_row(
            &format!("SELECT {ENTRY_COLUMNS} FROM entries WHERE id = ?1"),
            [id],
            read_entry,
        )
        .optional()?;
    Ok(entry)
}

pub fn list_by_date(connection: &Connection, entry_date: &str) -> AppResult<Vec<EntrySummary>> {
    let mut statement = connection.prepare(
        "SELECT id, entry_date, title, excerpt, cover_attachment_id, created_at_utc, updated_at_utc
         FROM entries
         WHERE entry_date = ?1 AND deleted_at_utc IS NULL
         ORDER BY created_at_utc ASC, id ASC",
    )?;
    let rows = statement.query_map([entry_date], |row| {
        Ok(EntrySummary {
            id: row.get(0)?,
            entry_date: row.get(1)?,
            title: row.get(2)?,
            excerpt: row.get(3)?,
            cover_attachment_id: row.get(4)?,
            created_at_utc: row.get(5)?,
            updated_at_utc: row.get(6)?,
        })
    })?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

/// 回收站：只列软删除的日记，最近删除的排在前面。
pub fn list_deleted(connection: &Connection) -> AppResult<Vec<DeletedEntry>> {
    let mut statement = connection.prepare(
        "SELECT id, entry_date, excerpt, cover_attachment_id, created_at_utc, deleted_at_utc
         FROM entries
         WHERE deleted_at_utc IS NOT NULL
         ORDER BY deleted_at_utc DESC, id DESC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(DeletedEntry {
            id: row.get(0)?,
            entry_date: row.get(1)?,
            excerpt: row.get(2)?,
            cover_attachment_id: row.get(3)?,
            created_at_utc: row.get(4)?,
            deleted_at_utc: row.get(5)?,
        })
    })?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

/// 软删除与恢复只动 deleted_at_utc，正文、附件和标签关系保持不变。
pub fn set_deleted_at(
    connection: &Connection,
    id: &str,
    deleted_at_utc: Option<&str>,
) -> AppResult<bool> {
    let changed = connection.execute(
        "UPDATE entries SET deleted_at_utc = ?2 WHERE id = ?1",
        rusqlite::params![id, deleted_at_utc],
    )?;
    Ok(changed == 1)
}

pub fn save(
    connection: &Connection,
    draft: &EntryDraft,
    excerpt: &str,
    now_utc: &str,
) -> AppResult<bool> {
    let changed = connection.execute(
        "UPDATE entries
         SET title = ?2, content_json = ?3, content_text = ?4, excerpt = ?5, cover_attachment_id = ?6, updated_at_utc = ?7
         WHERE id = ?1",
        rusqlite::params![
            draft.id,
            draft.title,
            draft.content_json,
            draft.content_text,
            excerpt,
            draft.cover_attachment_id,
            now_utc,
        ],
    )?;
    Ok(changed == 1)
}

pub fn require(connection: &Connection, id: &str) -> AppResult<Entry> {
    load(connection, id)?.ok_or_else(|| AppError::MissingEntry(id.to_string()))
}

/// 摘要由纯文本折叠空白后截取，避免前端传入超长或带换行的内容。
pub fn build_excerpt(content_text: &str) -> String {
    content_text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(80)
        .collect()
}
