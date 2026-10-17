use tauri::State;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::entries::{repository, Entry, EntryDraft, EntrySummary};
use crate::error::{AppError, AppResult};
use crate::library::{Library, LibraryInfo};

/// M1 的正文结构：一个空段落，带版本号供 M2 的编辑器升级。
const EMPTY_CONTENT_JSON: &str =
    r#"{"version":1,"type":"doc","content":[{"type":"paragraph","text":""}]}"#;

fn now_utc() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("UTC 时间格式化不会失败")
}

#[tauri::command]
pub fn library_info(library: State<'_, Library>) -> AppResult<LibraryInfo> {
    library.info()
}

#[tauri::command]
pub fn list_entries(library: State<'_, Library>, entry_date: String) -> AppResult<Vec<EntrySummary>> {
    library.with_connection(|connection| repository::list_by_date(connection, &entry_date))
}

#[tauri::command]
pub fn get_entry(library: State<'_, Library>, id: String) -> AppResult<Entry> {
    library.with_connection(|connection| repository::require(connection, &id))
}

#[tauri::command]
pub fn create_entry(library: State<'_, Library>, entry_date: String) -> AppResult<Entry> {
    let id = Uuid::new_v4().to_string();
    let now = now_utc();

    library.with_connection(|connection| {
        repository::create(connection, &id, &entry_date, EMPTY_CONTENT_JSON, &now)
    })
}

#[tauri::command]
pub fn save_entry(library: State<'_, Library>, draft: EntryDraft) -> AppResult<Entry> {
    let now = now_utc();
    let excerpt = repository::build_excerpt(&draft.content_text);

    library.with_connection(|connection| {
        if !repository::save(connection, &draft, &excerpt, &now)? {
            return Err(AppError::MissingEntry(draft.id.clone()));
        }
        repository::require(connection, &draft.id)
    })
}
