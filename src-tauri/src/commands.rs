use tauri::http::{header, Response, StatusCode};
use tauri::{Manager, State, UriSchemeContext};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::attachments::{self, repository as attachment_repository, Attachment};
use crate::entries::{repository, Entry, EntryDraft, EntrySummary};
use crate::error::{AppError, AppResult};
use crate::library::{Library, LibraryInfo};

/// 新日记的空正文：一个空段落。版本 2 对应 ADR-0004 的 Tiptap 文档格式。
const EMPTY_CONTENT_JSON: &str =
    r#"{"version":2,"type":"doc","content":[{"type":"paragraph"}]}"#;

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

/// 导入图片：先写原图和缩略图，再落元数据；任何一步失败都清掉已经写下的文件。
#[tauri::command]
pub fn import_attachment(
    library: State<'_, Library>,
    entry_id: String,
    bytes: Vec<u8>,
) -> AppResult<Attachment> {
    let info = attachments::inspect_image(&bytes)?;
    let thumbnail = attachments::build_thumbnail(&bytes)?;
    let id = Uuid::new_v4().to_string();
    let relative = attachments::original_path(&entry_id, &id, info.extension);
    let thumbnail_relative = attachments::thumbnail_path(&id);
    let root = library.root();

    attachments::write_file(root, &relative, &bytes)?;
    if let Err(error) = attachments::write_file(root, &thumbnail_relative, &thumbnail) {
        attachments::remove_file(root, &relative);
        return Err(error);
    }

    let attachment = Attachment {
        id,
        entry_id: entry_id.clone(),
        relative_path: relative.clone(),
        mime_type: info.mime_type.to_string(),
        file_size: bytes.len() as i64,
        sha256: attachments::sha256_hex(&bytes),
        width: info.width,
        height: info.height,
        created_at_utc: now_utc(),
    };

    let inserted = library.with_connection(|connection| {
        attachment_repository::insert(connection, &attachment)
    });
    if let Err(error) = inserted {
        attachments::remove_file(root, &relative);
        attachments::remove_file(root, &thumbnail_relative);
        return Err(error);
    }

    Ok(attachment)
}

/// 删除附件：先删元数据，再删文件。正文里的引用由前端在调用前移除。
#[tauri::command]
pub fn delete_attachment(library: State<'_, Library>, id: String) -> AppResult<()> {
    let relative = library.with_connection(|connection| attachment_repository::delete(connection, &id))?;

    if let Some(relative) = relative {
        attachments::remove_file(library.root(), &relative);
        attachments::remove_file(library.root(), &attachments::thumbnail_path(&id));
    }
    Ok(())
}

/// 自定义协议 `attachment://`：按附件 ID 把原图读给编辑器。
pub fn attachment_response(
    context: UriSchemeContext<'_, tauri::Wry>,
    request: tauri::http::Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let id = request.uri().path().trim_start_matches('/');
    let library = context.app_handle().state::<Library>();

    let found = library.with_connection(|connection| attachment_repository::find_for_read(connection, id));
    let response = match found {
        Ok(Some((relative, mime_type))) => std::fs::read(library.root().join(relative))
            .map(|bytes| {
                Response::builder()
                    .header(header::CONTENT_TYPE, mime_type)
                    .body(bytes)
                    .expect("附件响应可以构建")
            })
            .unwrap_or_else(|error| error_response(error.to_string())),
        Ok(None) => error_response("附件不存在".to_string()),
        Err(error) => error_response(error.to_string()),
    };
    response
}

fn error_response(message: String) -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(message.into_bytes())
        .expect("错误响应可以构建")
}
