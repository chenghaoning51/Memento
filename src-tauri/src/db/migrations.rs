use crate::error::{AppError, AppResult};
use rusqlite::Connection;

/// 按顺序应用的数据库迁移。索引 + 1 就是写入 `PRAGMA user_version` 的版本号。
const MIGRATIONS: &[&str] = &[
    // v1：日记主体。附件、标签、信件在 M2–M4 通过新增迁移加入。
    "CREATE TABLE entries (
        id TEXT PRIMARY KEY NOT NULL,
        entry_date TEXT NOT NULL,
        title TEXT NOT NULL DEFAULT '',
        content_json TEXT NOT NULL,
        content_text TEXT NOT NULL DEFAULT '',
        excerpt TEXT NOT NULL DEFAULT '',
        layout_style TEXT NOT NULL DEFAULT 'default',
        cover_attachment_id TEXT,
        created_at_utc TEXT NOT NULL,
        updated_at_utc TEXT NOT NULL,
        deleted_at_utc TEXT
     );
     CREATE INDEX entries_by_date ON entries (entry_date, created_at_utc);
     CREATE INDEX entries_by_deleted ON entries (deleted_at_utc);",
];

pub fn target_version() -> i64 {
    MIGRATIONS.len() as i64
}

pub fn schema_version(conn: &Connection) -> AppResult<i64> {
    let version = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    Ok(version)
}

/// 在单个事务内把数据库升级到目标版本；任何一步失败都回滚，旧数据保持不变。
pub fn migrate(conn: &mut Connection) -> AppResult<i64> {
    let current = schema_version(conn)?;
    let target = target_version();

    if current > target {
        return Err(AppError::UnsupportedSchema(current));
    }

    for (index, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let version = index as i64 + 1;
        let transaction = conn.transaction()?;
        transaction.execute_batch(sql)?;
        transaction.pragma_update(None, "user_version", version)?;
        transaction.commit()?;
    }

    Ok(target)
}
