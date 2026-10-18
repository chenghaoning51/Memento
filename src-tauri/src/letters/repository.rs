use rusqlite::{Connection, OptionalExtension};

use super::{Letter, LetterDraft, STATUS_OPENED};
use crate::error::AppResult;

const LETTER_COLUMNS: &str = "id, title, content_json, content_text, unlock_at_utc, unlock_timezone, status, reply_json, reply_text, opened_at_utc, created_at_utc, updated_at_utc";

fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Letter> {
    Ok(Letter {
        id: row.get(0)?,
        title: row.get(1)?,
        content_json: row.get(2)?,
        content_text: row.get(3)?,
        unlock_at_utc: row.get(4)?,
        unlock_timezone: row.get(5)?,
        status: row.get(6)?,
        reply_json: row.get(7)?,
        reply_text: row.get(8)?,
        opened_at_utc: row.get(9)?,
        created_at_utc: row.get(10)?,
        updated_at_utc: row.get(11)?,
    })
}

/// 信件列表按开启时刻排序：最早到期的最先出现，还没排上的在后面。
pub fn list(connection: &Connection) -> AppResult<Vec<Letter>> {
    let mut statement = connection.prepare(&format!(
        "SELECT {LETTER_COLUMNS} FROM letters ORDER BY unlock_at_utc ASC, created_at_utc ASC, id ASC"
    ))?;
    let letters = statement
        .query_map([], from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(letters)
}

pub fn find(connection: &Connection, id: &str) -> AppResult<Option<Letter>> {
    let letter = connection
        .query_row(
            &format!("SELECT {LETTER_COLUMNS} FROM letters WHERE id = ?1"),
            [id],
            from_row,
        )
        .optional()?;
    Ok(letter)
}

pub fn insert(connection: &Connection, id: &str, draft: &LetterDraft, status: &str, now_utc: &str) -> AppResult<()> {
    connection.execute(
        "INSERT INTO letters (id, title, content_json, content_text, unlock_at_utc, unlock_timezone, status, created_at_utc, updated_at_utc)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        rusqlite::params![
            id,
            draft.title,
            draft.content_json,
            draft.content_text,
            draft.unlock_at_utc,
            draft.unlock_timezone,
            status,
            now_utc
        ],
    )?;
    Ok(())
}

/// 到期后原信不再可编辑，状态列也不回退；这里的 UPDATE 只用于锁定期间改信。
pub fn update(connection: &Connection, id: &str, draft: &LetterDraft, status: &str, now_utc: &str) -> AppResult<bool> {
    let changed = connection.execute(
        "UPDATE letters SET title = ?2, content_json = ?3, content_text = ?4, unlock_at_utc = ?5, unlock_timezone = ?6, status = ?7, updated_at_utc = ?8
         WHERE id = ?1",
        rusqlite::params![
            id,
            draft.title,
            draft.content_json,
            draft.content_text,
            draft.unlock_at_utc,
            draft.unlock_timezone,
            status,
            now_utc
        ],
    )?;
    Ok(changed > 0)
}

pub fn delete(connection: &Connection, id: &str) -> AppResult<bool> {
    let changed = connection.execute("DELETE FROM letters WHERE id = ?1", [id])?;
    Ok(changed > 0)
}

/// 打开原信：记下 opened_at_utc 并进入 opened；已经读过的信不改动打开时间。
pub fn mark_opened(connection: &Connection, id: &str, now_utc: &str) -> AppResult<()> {
    connection.execute(
        "UPDATE letters SET status = ?2, opened_at_utc = ?3, updated_at_utc = ?3 WHERE id = ?1 AND opened_at_utc IS NULL",
        rusqlite::params![id, STATUS_OPENED, now_utc],
    )?;
    Ok(())
}

/// 回应独立保存，原信的标题与正文不受影响。
pub fn save_reply(connection: &Connection, id: &str, reply_json: &str, reply_text: &str, now_utc: &str) -> AppResult<bool> {
    let changed = connection.execute(
        "UPDATE letters SET reply_json = ?2, reply_text = ?3, updated_at_utc = ?4 WHERE id = ?1",
        rusqlite::params![id, reply_json, reply_text, now_utc],
    )?;
    Ok(changed > 0)
}
