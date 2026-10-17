use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

use super::{clean, normalized, Tag};
use crate::error::AppResult;

/// 一篇日记的标签，按加入顺序排列。
pub fn list_for_entry(connection: &Connection, entry_id: &str) -> AppResult<Vec<Tag>> {
    let mut statement = connection.prepare(
        "SELECT t.id, t.name
         FROM entry_tags et JOIN tags t ON t.id = et.tag_id
         WHERE et.entry_id = ?1
         ORDER BY et.rowid ASC",
    )?;
    let rows = statement.query_map([entry_id], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?;

    let mut tags = Vec::new();
    for row in rows {
        tags.push(row?);
    }
    Ok(tags)
}

/// 加标签：同名（规范化后）的标签复用已有记录，重复加入同一篇日记时保持幂等。
pub fn add(connection: &Connection, entry_id: &str, name: &str) -> AppResult<()> {
    let normalized_name = normalized(name);
    let existing: Option<String> = connection
        .query_row(
            "SELECT id FROM tags WHERE normalized_name = ?1",
            [&normalized_name],
            |row| row.get(0),
        )
        .optional()?;

    let tag_id = match existing {
        Some(id) => id,
        None => {
            let id = Uuid::new_v4().to_string();
            connection.execute(
                "INSERT INTO tags (id, name, normalized_name) VALUES (?1, ?2, ?3)",
                rusqlite::params![id, clean(name), normalized_name],
            )?;
            id
        }
    };

    connection.execute(
        "INSERT OR IGNORE INTO entry_tags (entry_id, tag_id) VALUES (?1, ?2)",
        rusqlite::params![entry_id, tag_id],
    )?;
    Ok(())
}

pub fn remove(connection: &Connection, entry_id: &str, tag_id: &str) -> AppResult<()> {
    connection.execute(
        "DELETE FROM entry_tags WHERE entry_id = ?1 AND tag_id = ?2",
        rusqlite::params![entry_id, tag_id],
    )?;
    Ok(())
}
