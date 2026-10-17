use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::Connection;
use serde::Serialize;

use crate::db::migrations;
use crate::error::AppResult;

/// 应用数据目录里的 Library 根：数据库、附件、缩略图、备份和导出都放在这里。
pub struct Library {
    root: PathBuf,
    connection: Mutex<Connection>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryInfo {
    pub root: PathBuf,
    pub database_path: PathBuf,
    pub schema_version: i64,
}

impl Library {
    pub fn open(root: PathBuf) -> AppResult<Self> {
        std::fs::create_dir_all(&root)?;

        let database_path = root.join("memento.db");
        let mut connection = Connection::open(&database_path)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;

        migrations::migrate(&mut connection)?;

        Ok(Self {
            root,
            connection: Mutex::new(connection),
        })
    }

    pub fn with_connection<T>(&self, action: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let connection = self.connection.lock().expect("library connection poisoned");
        action(&connection)
    }

    pub fn info(&self) -> AppResult<LibraryInfo> {
        let schema_version = self.with_connection(migrations::schema_version)?;

        Ok(LibraryInfo {
            root: self.root.clone(),
            database_path: self.root.join("memento.db"),
            schema_version,
        })
    }
}
