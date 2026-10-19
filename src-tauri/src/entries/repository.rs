use rusqlite::{Connection, OptionalExtension};

use super::{DeletedEntry, Entry, EntryDateSummary, EntryDraft, EntrySummary, SearchQuery};
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

/// 月历用：区间内每个有记录的本地日期、当天篇数与首图。
pub fn list_dates(
    connection: &Connection,
    start_date: &str,
    end_date: &str,
) -> AppResult<Vec<EntryDateSummary>> {
    let mut statement = connection.prepare(
        "SELECT entry_date,
                COUNT(*),
                (SELECT cover_attachment_id
                 FROM entries AS inner_entry
                 WHERE inner_entry.entry_date = outer_entry.entry_date
                   AND inner_entry.deleted_at_utc IS NULL
                   AND inner_entry.cover_attachment_id IS NOT NULL
                 ORDER BY inner_entry.created_at_utc ASC, inner_entry.id ASC
                 LIMIT 1)
         FROM entries AS outer_entry
         WHERE entry_date BETWEEN ?1 AND ?2 AND deleted_at_utc IS NULL
         GROUP BY entry_date
         ORDER BY entry_date ASC",
    )?;
    let rows = statement.query_map([start_date, end_date], |row| {
        Ok(EntryDateSummary {
            entry_date: row.get(0)?,
            entry_count: row.get(1)?,
            cover_attachment_id: row.get(2)?,
        })
    })?;

    let mut dates = Vec::new();
    for row in rows {
        dates.push(row?);
    }
    Ok(dates)
}

/// 时间线：按日记日期倒序、同一天内后写的在前。
pub fn list_recent(
    connection: &Connection,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<EntrySummary>> {
    let mut statement = connection.prepare(
        "SELECT id, entry_date, title, excerpt, cover_attachment_id, created_at_utc, updated_at_utc
         FROM entries
         WHERE deleted_at_utc IS NULL
         ORDER BY entry_date DESC, created_at_utc DESC, id DESC
         LIMIT ?1 OFFSET ?2",
    )?;
    let rows = statement.query_map([limit, offset], |row| {
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
        "SELECT id, entry_date, title, excerpt, cover_attachment_id, created_at_utc, deleted_at_utc
         FROM entries
         WHERE deleted_at_utc IS NOT NULL
         ORDER BY deleted_at_utc DESC, id DESC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(DeletedEntry {
            id: row.get(0)?,
            entry_date: row.get(1)?,
            title: row.get(2)?,
            excerpt: row.get(3)?,
            cover_attachment_id: row.get(4)?,
            created_at_utc: row.get(5)?,
            deleted_at_utc: row.get(6)?,
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

/// 永久删除一篇回收站里的日记：正文、附件行与标签关系一起消失。
/// 只接受已经软删除的记录，正常日记必须先经过回收站。
/// 返回原图相对路径，调用方据此删除磁盘文件。
pub fn purge(connection: &Connection, id: &str) -> AppResult<Vec<String>> {
    let deleted_at: Option<Option<String>> = connection
        .query_row(
            "SELECT deleted_at_utc FROM entries WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .optional()?;
    match deleted_at {
        None => return Err(AppError::MissingEntry(id.to_string())),
        Some(None) => return Err(AppError::EntryNotInTrash),
        Some(Some(_)) => {}
    }

    let paths = attachment_paths(connection, id)?;
    let transaction = connection.unchecked_transaction()?;
    transaction.execute("DELETE FROM entry_tags WHERE entry_id = ?1", [id])?;
    transaction.execute("DELETE FROM attachments WHERE entry_id = ?1", [id])?;
    transaction.execute("DELETE FROM entries WHERE id = ?1", [id])?;
    transaction.execute(
        "DELETE FROM tags WHERE id NOT IN (SELECT tag_id FROM entry_tags)",
        [],
    )?;
    transaction.commit()?;
    Ok(paths)
}

/// 清空回收站：所有软删除的日记连同附件行、标签关系与孤儿标签一起删掉。
/// 返回删掉的篇数与原图相对路径。
pub fn purge_deleted(connection: &Connection) -> AppResult<(usize, Vec<String>)> {
    let mut statement = connection.prepare("SELECT id FROM entries WHERE deleted_at_utc IS NOT NULL")?;
    let ids: Vec<String> = statement
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);

    let mut paths = Vec::new();
    for id in &ids {
        paths.extend(attachment_paths(connection, id)?);
    }

    let transaction = connection.unchecked_transaction()?;
    transaction.execute_batch(
        "DELETE FROM entry_tags WHERE entry_id IN (SELECT id FROM entries WHERE deleted_at_utc IS NOT NULL);
         DELETE FROM attachments WHERE entry_id IN (SELECT id FROM entries WHERE deleted_at_utc IS NOT NULL);
         DELETE FROM entries WHERE deleted_at_utc IS NOT NULL;
         DELETE FROM tags WHERE id NOT IN (SELECT tag_id FROM entry_tags);",
    )?;
    transaction.commit()?;
    Ok((ids.len(), paths))
}

/// 一篇日记的全部原图相对路径；缩略图路径由附件 ID 推导。
fn attachment_paths(connection: &Connection, entry_id: &str) -> AppResult<Vec<String>> {
    let mut statement =
        connection.prepare("SELECT relative_path FROM attachments WHERE entry_id = ?1")?;
    let paths = statement
        .query_map([entry_id], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(paths)
}

pub fn save(
    connection: &Connection,
    draft: &EntryDraft,
    excerpt: &str,
    search_text: &str,
    now_utc: &str,
) -> AppResult<bool> {
    let changed = connection.execute(
        "UPDATE entries
         SET title = ?2, content_json = ?3, content_text = ?4, excerpt = ?5, cover_attachment_id = ?6,
             search_text = ?7, updated_at_utc = ?8
         WHERE id = ?1",
        rusqlite::params![
            draft.id,
            draft.title,
            draft.content_json,
            draft.content_text,
            excerpt,
            draft.cover_attachment_id,
            search_text,
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

/// 搜索列：标题与正文折叠空白后转小写，用换行分隔，避免标题末字与正文首字拼成一个词。
pub fn search_text(title: &str, content_text: &str) -> String {
    let title = collapse(title);
    let body = collapse(content_text);
    match (title.is_empty(), body.is_empty()) {
        (true, _) => body,
        (false, true) => title,
        (false, false) => format!("{title}\n{body}"),
    }
}

fn collapse(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// LIKE 里的特殊字符要转义，否则用户输入的 % 和 _ 会变成通配符。
fn escape_like(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// 搜索：关键词子串 + 标签交集 + 日期范围，按日期倒序分批取。
pub fn search(connection: &Connection, query: &SearchQuery) -> AppResult<Vec<EntrySummary>> {
    let mut sql = String::from(
        "SELECT id, entry_date, title, excerpt, cover_attachment_id, created_at_utc, updated_at_utc
         FROM entries
         WHERE deleted_at_utc IS NULL",
    );
    let mut values: Vec<rusqlite::types::Value> = Vec::new();

    let text = collapse(&query.text);
    if !text.is_empty() {
        values.push(rusqlite::types::Value::Text(format!("%{}%", escape_like(&text))));
        sql.push_str(&format!(" AND search_text LIKE ?{} ESCAPE '\\'", values.len()));
    }

    if !query.start_date.is_empty() {
        values.push(rusqlite::types::Value::Text(query.start_date.clone()));
        sql.push_str(&format!(" AND entry_date >= ?{}", values.len()));
    }

    if !query.end_date.is_empty() {
        values.push(rusqlite::types::Value::Text(query.end_date.clone()));
        sql.push_str(&format!(" AND entry_date <= ?{}", values.len()));
    }

    if !query.tag_ids.is_empty() {
        let mut placeholders = Vec::new();
        for tag_id in &query.tag_ids {
            values.push(rusqlite::types::Value::Text(tag_id.clone()));
            placeholders.push(format!("?{}", values.len()));
        }
        sql.push_str(&format!(
            " AND (SELECT COUNT(DISTINCT et.tag_id) FROM entry_tags et
                   WHERE et.entry_id = entries.id AND et.tag_id IN ({})) = {}",
            placeholders.join(", "),
            query.tag_ids.len()
        ));
    }

    values.push(rusqlite::types::Value::Integer(query.limit));
    let limit_index = values.len();
    values.push(rusqlite::types::Value::Integer(query.offset));
    let offset_index = values.len();
    sql.push_str(&format!(
        " ORDER BY entry_date DESC, created_at_utc DESC, id DESC LIMIT ?{limit_index} OFFSET ?{offset_index}"
    ));

    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(rusqlite::params_from_iter(values.iter()), |row| {
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

/// 迁移到 v4 后回填旧日记的搜索列。只跑一次，之后由 save 维护。
pub fn rebuild_search_text(connection: &Connection) -> AppResult<()> {
    let mut statement = connection.prepare("SELECT id, title, content_text FROM entries")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    let mut updates = Vec::new();
    for row in rows {
        updates.push(row?);
    }
    for (id, title, content_text) in updates {
        connection.execute(
            "UPDATE entries SET search_text = ?2 WHERE id = ?1",
            rusqlite::params![id, search_text(&title, &content_text)],
        )?;
    }
    Ok(())
}
