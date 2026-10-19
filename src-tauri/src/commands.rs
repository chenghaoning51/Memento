use std::path::{Path, PathBuf};

use tauri::http::{header, Response, StatusCode};
use tauri::{Manager, State, UriSchemeContext};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::attachments::{self, repository as attachment_repository, Attachment};
use crate::backup;
use crate::entries::{
    repository, DeletedEntry, Entry, EntryDateSummary, EntryDraft, EntrySummary, SearchQuery,
};
use crate::error::{AppError, AppResult};
use crate::export;
use crate::letters::{self, repository as letter_repository, Letter, LetterDraft};
use crate::library::{Library, LibraryInfo};
use crate::location;
use crate::LocationState;
use crate::tags::{self, repository as tag_repository, Tag};

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

/// 把整个空间搬到新目录：复制数据库、附件、缩略图与自动备份，再记下新位置。
/// 复制完成后需要重启应用，当前连接仍指向旧目录。
#[tauri::command]
pub fn relocate_library(
    library: State<'_, Library>,
    location: State<'_, LocationState>,
    directory: String,
) -> AppResult<()> {
    let target = PathBuf::from(directory);
    if target == library.root() {
        return Ok(());
    }
    location::copy_space(library.root(), &target)?;
    location::set_configured_root(&location.config_dir, Some(&target))
}

/// 回到默认数据目录。只改配置，不搬文件；重启后生效。
#[tauri::command]
pub fn reset_library_location(location: State<'_, LocationState>) -> AppResult<()> {
    location::set_configured_root(&location.config_dir, None)
}

#[tauri::command]
pub fn list_entries(library: State<'_, Library>, entry_date: String) -> AppResult<Vec<EntrySummary>> {
    library.with_connection(|connection| repository::list_by_date(connection, &entry_date))
}

/// 月历用：某个月内每个有记录的日期。日期是本地日历日期，按字符串区间比较。
#[tauri::command]
pub fn list_entry_dates(
    library: State<'_, Library>,
    start_date: String,
    end_date: String,
) -> AppResult<Vec<EntryDateSummary>> {
    library.with_connection(|connection| repository::list_dates(connection, &start_date, &end_date))
}

/// 时间线用：最近的日记，按日期倒序分批取。
#[tauri::command]
pub fn list_timeline(
    library: State<'_, Library>,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<EntrySummary>> {
    library.with_connection(|connection| repository::list_recent(connection, limit, offset))
}

/// 搜索：关键词子串、标签交集与日期范围可以任意组合，结果按日期倒序分批返回。
#[tauri::command]
pub fn search_entries(library: State<'_, Library>, query: SearchQuery) -> AppResult<Vec<EntrySummary>> {
    library.with_connection(|connection| repository::search(connection, &query))
}

#[tauri::command]
pub fn list_tags(library: State<'_, Library>) -> AppResult<Vec<Tag>> {
    library.with_connection(tag_repository::list_all)
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
    let search_text = repository::search_text(&draft.title, &draft.content_text);

    library.with_connection(|connection| {
        if !repository::save(connection, &draft, &excerpt, &search_text, &now)? {
            return Err(AppError::MissingEntry(draft.id.clone()));
        }
        repository::require(connection, &draft.id)
    })
}

#[tauri::command]
pub fn list_deleted_entries(library: State<'_, Library>) -> AppResult<Vec<DeletedEntry>> {
    library.with_connection(repository::list_deleted)
}

/// 软删除：只写 deleted_at_utc，正文、附件和标签关系都不动。
#[tauri::command]
pub fn delete_entry(library: State<'_, Library>, id: String) -> AppResult<()> {
    let now = now_utc();
    library.with_connection(|connection| {
        if !repository::set_deleted_at(connection, &id, Some(&now))? {
            return Err(AppError::MissingEntry(id.clone()));
        }
        Ok(())
    })
}

#[tauri::command]
pub fn restore_entry(library: State<'_, Library>, id: String) -> AppResult<Entry> {
    library.with_connection(|connection| {
        if !repository::set_deleted_at(connection, &id, None)? {
            return Err(AppError::MissingEntry(id.clone()));
        }
        repository::require(connection, &id)
    })
}

/// 永久删除回收站里的一篇日记：先删数据库行，再删磁盘上的原图与缩略图。
#[tauri::command]
pub fn purge_entry(library: State<'_, Library>, id: String) -> AppResult<()> {
    let paths = library.with_connection(|connection| repository::purge(connection, &id))?;
    remove_attachment_files(library.root(), &paths);
    Ok(())
}

/// 清空回收站：返回删掉了几篇日记。
#[tauri::command]
pub fn purge_trash(library: State<'_, Library>) -> AppResult<usize> {
    let (count, paths) = library.with_connection(repository::purge_deleted)?;
    remove_attachment_files(library.root(), &paths);
    Ok(count)
}

/// 附件文件按相对路径删除，缩略图路径由文件名里的附件 ID 推导。
fn remove_attachment_files(root: &Path, paths: &[String]) {
    for relative in paths {
        attachments::remove_file(root, relative);
        if let Some(stem) = Path::new(relative).file_stem().and_then(|value| value.to_str()) {
            attachments::remove_file(root, &attachments::thumbnail_path(stem));
        }
    }
}

#[tauri::command]
pub fn list_entry_tags(library: State<'_, Library>, entry_id: String) -> AppResult<Vec<Tag>> {
    library.with_connection(|connection| tag_repository::list_for_entry(connection, &entry_id))
}

/// 加标签时先规范化再写库：空白与大小写不影响去重，同一篇日记不会重复关联。
#[tauri::command]
pub fn add_entry_tag(
    library: State<'_, Library>,
    entry_id: String,
    name: String,
) -> AppResult<Vec<Tag>> {
    let clean = tags::clean(&name);
    if clean.chars().count() > tags::MAX_TAG_CHARS {
        return Err(AppError::TagTooLong);
    }
    if clean.is_empty() {
        return library.with_connection(|connection| {
            tag_repository::list_for_entry(connection, &entry_id)
        });
    }

    library.with_connection(|connection| {
        tag_repository::add(connection, &entry_id, &clean)?;
        tag_repository::list_for_entry(connection, &entry_id)
    })
}

#[tauri::command]
pub fn remove_entry_tag(
    library: State<'_, Library>,
    entry_id: String,
    tag_id: String,
) -> AppResult<Vec<Tag>> {
    library.with_connection(|connection| {
        tag_repository::remove(connection, &entry_id, &tag_id)?;
        tag_repository::list_for_entry(connection, &entry_id)
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

#[tauri::command]
pub fn list_letters(library: State<'_, Library>) -> AppResult<Vec<Letter>> {
    let now = now_utc();
    library.with_connection(|connection| {
        let mut letters = letter_repository::list(connection)?;
        // 状态在每次读取时按当前 UTC 推进：应用关着的时候到期，重开进页面就正确。
        for letter in letters.iter_mut() {
            if letter.status == letters::STATUS_LOCKED && now >= letter.unlock_at_utc {
                letter.status = letters::STATUS_UNLOCKED.to_string();
            }
        }
        Ok(letters)
    })
}

#[tauri::command]
pub fn get_letter(library: State<'_, Library>, id: String) -> AppResult<Letter> {
    let now = now_utc();
    library.with_connection(|connection| {
        let mut letter =
            letter_repository::find(connection, &id)?.ok_or_else(|| AppError::MissingLetter(id.clone()))?;
        if letter.status == letters::STATUS_LOCKED && now >= letter.unlock_at_utc {
            letter.status = letters::STATUS_UNLOCKED.to_string();
        }
        Ok(letter)
    })
}

#[tauri::command]
pub fn create_letter(library: State<'_, Library>, draft: LetterDraft) -> AppResult<Letter> {
    let id = Uuid::new_v4().to_string();
    let now = now_utc();
    let status = letters::status_at(&draft.unlock_at_utc, &now);

    library.with_connection(|connection| {
        letter_repository::insert(connection, &id, &draft, status, &now)?;
        letter_repository::find(connection, &id)?
            .ok_or_else(|| AppError::MissingLetter(id.clone()))
    })
}

/// 改信只在到期前允许：已经能读的信不能回头改原文，只能留回应。
#[tauri::command]
pub fn update_letter(library: State<'_, Library>, id: String, draft: LetterDraft) -> AppResult<Letter> {
    let now = now_utc();
    let status = letters::status_at(&draft.unlock_at_utc, &now);

    library.with_connection(|connection| {
        let existing =
            letter_repository::find(connection, &id)?.ok_or_else(|| AppError::MissingLetter(id.clone()))?;
        // 只看状态列不够：到期但还没打开过的信，库里的状态仍是 locked，同样不能再改原文。
        if existing.status != letters::STATUS_LOCKED || now >= existing.unlock_at_utc {
            return Err(AppError::LetterExpired);
        }
        if !letter_repository::update(connection, &id, &draft, status, &now)? {
            return Err(AppError::MissingLetter(id.clone()));
        }
        letter_repository::find(connection, &id)?
            .ok_or_else(|| AppError::MissingLetter(id.clone()))
    })
}

#[tauri::command]
pub fn delete_letter(library: State<'_, Library>, id: String) -> AppResult<()> {
    library.with_connection(|connection| {
        if !letter_repository::delete(connection, &id)? {
            return Err(AppError::MissingLetter(id.clone()));
        }
        Ok(())
    })
}

/// 打开原信：只有到期之后才能读，读到就记下打开时间。
#[tauri::command]
pub fn open_letter(library: State<'_, Library>, id: String) -> AppResult<Letter> {
    let now = now_utc();
    library.with_connection(|connection| {
        let letter =
            letter_repository::find(connection, &id)?.ok_or_else(|| AppError::MissingLetter(id.clone()))?;
        if now < letter.unlock_at_utc {
            return Err(AppError::LetterLocked);
        }
        letter_repository::mark_opened(connection, &id, &now)?;
        letter_repository::find(connection, &id)?
            .ok_or_else(|| AppError::MissingLetter(id.clone()))
    })
}

#[tauri::command]
pub fn save_letter_reply(
    library: State<'_, Library>,
    id: String,
    reply_json: String,
    reply_text: String,
) -> AppResult<Letter> {
    let now = now_utc();
    library.with_connection(|connection| {
        let letter =
            letter_repository::find(connection, &id)?.ok_or_else(|| AppError::MissingLetter(id.clone()))?;
        if now < letter.unlock_at_utc {
            return Err(AppError::LetterLocked);
        }
        if !letter_repository::save_reply(connection, &id, &reply_json, &reply_text, &now)? {
            return Err(AppError::MissingLetter(id.clone()));
        }
        letter_repository::find(connection, &id)?
            .ok_or_else(|| AppError::MissingLetter(id.clone()))
    })
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

/// 导出日记：`entry_ids` 为空表示全部，否则只导出选中的那些。
#[tauri::command]
pub fn export_entries(
    library: State<'_, Library>,
    directory: String,
    entry_ids: Vec<String>,
) -> AppResult<export::ExportSummary> {
    let target = PathBuf::from(directory);
    library.with_connection(|connection| {
        let entries = export::repository::list_for_export(connection, &entry_ids)?;
        let attachments = export::repository::attachment_paths(connection)?;
        export::write_entries(&target, library.root(), &entries, &attachments)
    })
}

/// 备份整个空间到用户选定的父目录，自动创建带时间戳的子目录。
#[tauri::command]
pub fn create_backup(
    library: State<'_, Library>,
    parent: String,
    app: tauri::AppHandle,
) -> AppResult<backup::BackupSummary> {
    let version = app.package_info().version.to_string();
    let now = now_utc();
    library.with_connection(|connection| {
        backup::create(connection, library.root(), Path::new(&parent), &now, &version)
    })
}

/// 恢复前预检：版本、清单与校验值。
#[tauri::command]
pub fn inspect_backup(directory: String) -> AppResult<backup::BackupInfo> {
    backup::inspect(Path::new(&directory))
}

/// 用备份替换当前空间；替换前会先备份当前空间。
#[tauri::command]
pub fn restore_backup(
    library: State<'_, Library>,
    directory: String,
    app: tauri::AppHandle,
) -> AppResult<backup::RestoreSummary> {
    let version = app.package_info().version.to_string();
    let now = now_utc();
    library.with_connection(|connection| {
        backup::restore(connection, library.root(), Path::new(&directory), &now, &version)
    })
}

fn error_response(message: String) -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(message.into_bytes())
        .expect("错误响应可以构建")
}
