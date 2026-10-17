use std::path::PathBuf;

use rusqlite::Connection;
use uuid::Uuid;

use crate::db::migrations;
use crate::entries::{repository, EntryDraft};
use crate::error::AppError;
use crate::library::Library;

const EMPTY_CONTENT_JSON: &str =
    r#"{"version":2,"type":"doc","content":[{"type":"paragraph"}]}"#;
const CREATED_AT: &str = "2026-10-17T08:00:00Z";

fn temp_library_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("memento-test-{name}-{}", Uuid::new_v4()))
}

/// A01：首次启动建库并迁移到当前版本；再次启动不重复迁移。
#[test]
fn creates_database_once_and_keeps_schema_version() {
    let root = temp_library_root("reopen");

    let first = Library::open(root.clone()).unwrap().info().unwrap();
    assert_eq!(first.schema_version, migrations::target_version());
    assert!(first.database_path.exists());

    let second = Library::open(root.clone()).unwrap().info().unwrap();
    assert_eq!(second.schema_version, migrations::target_version());

    std::fs::remove_dir_all(&root).ok();
}

/// A02：中文正文写入磁盘，关闭后重新打开仍然一致。
#[test]
fn saved_entry_survives_reopen() {
    let root = temp_library_root("entry");
    let id = Uuid::new_v4().to_string();
    let text = "今天下雨了。\n\n窗外的梧桐叶被打湿。";

    {
        let library = Library::open(root.clone()).unwrap();
        library
            .with_connection(|connection| {
                repository::create(connection, &id, "2026-10-17", EMPTY_CONTENT_JSON, CREATED_AT)
            })
            .unwrap();

        let draft = EntryDraft {
            id: id.clone(),
            title: String::new(),
            content_json: EMPTY_CONTENT_JSON.to_string(),
            content_text: text.to_string(),
        };
        let excerpt = repository::build_excerpt(&draft.content_text);
        let saved = library
            .with_connection(|connection| {
                repository::save(connection, &draft, &excerpt, "2026-10-17T08:01:00Z")
            })
            .unwrap();
        assert!(saved);
    }

    let reopened = Library::open(root.clone()).unwrap();
    let entry = reopened
        .with_connection(|connection| repository::require(connection, &id))
        .unwrap();

    assert_eq!(entry.content_text, text);
    assert_eq!(entry.excerpt, "今天下雨了。 窗外的梧桐叶被打湿。");
    assert_eq!(entry.entry_date, "2026-10-17");
    assert_eq!(entry.updated_at_utc, "2026-10-17T08:01:00Z");

    std::fs::remove_dir_all(&root).ok();
}

/// A15：数据库模式版本高于当前应用时拒绝打开，不写入也不降级。
#[test]
fn refuses_newer_schema_version() {
    let root = temp_library_root("future");
    std::fs::create_dir_all(&root).unwrap();

    {
        let connection = Connection::open(root.join("memento.db")).unwrap();
        connection
            .pragma_update(None, "user_version", migrations::target_version() + 1)
            .unwrap();
    }

    assert!(matches!(
        Library::open(root.clone()),
        Err(AppError::UnsupportedSchema(_))
    ));

    std::fs::remove_dir_all(&root).ok();
}
