use std::path::PathBuf;

use image::{EncodableLayout, ImageEncoder};
use rusqlite::Connection;
use uuid::Uuid;

use crate::attachments::{self, repository as attachment_repository, Attachment};
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
            cover_attachment_id: None,
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

/// A07：图片按内容识别，复制进 Library 后能按附件 ID 读回同一份字节。
#[test]
fn imported_attachment_is_readable_from_library_root() {
    let root = temp_library_root("attachment");
    let library = Library::open(root.clone()).unwrap();
    let entry_id = Uuid::new_v4().to_string();
    let attachment_id = Uuid::new_v4().to_string();

    // 附件行有外键约束，先落一条日记。
    library
        .with_connection(|connection| {
            repository::create(connection, &entry_id, "2026-10-17", EMPTY_CONTENT_JSON, CREATED_AT)
        })
        .unwrap();

    let bytes = sample_png(4, 3);
    let info = attachments::inspect_image(&bytes).unwrap();
    assert_eq!(info.extension, "png");
    assert_eq!((info.width, info.height), (4, 3));

    let relative = attachments::original_path(&entry_id, &attachment_id, info.extension);
    let thumbnail = attachments::build_thumbnail(&bytes).unwrap();
    attachments::write_file(library.root(), &relative, &bytes).unwrap();
    attachments::write_file(
        library.root(),
        &attachments::thumbnail_path(&attachment_id),
        &thumbnail,
    )
    .unwrap();

    let attachment = Attachment {
        id: attachment_id.clone(),
        entry_id: entry_id.clone(),
        relative_path: relative.clone(),
        mime_type: info.mime_type.to_string(),
        file_size: bytes.len() as i64,
        sha256: attachments::sha256_hex(&bytes),
        width: info.width,
        height: info.height,
        created_at_utc: CREATED_AT.to_string(),
    };
    library
        .with_connection(|connection| attachment_repository::insert(connection, &attachment))
        .unwrap();

    let (stored_relative, stored_mime) = library
        .with_connection(|connection| attachment_repository::find_for_read(connection, &attachment_id))
        .unwrap()
        .unwrap();
    assert_eq!(stored_relative, relative);
    assert_eq!(stored_mime, "image/png");
    assert_eq!(std::fs::read(library.root().join(&stored_relative)).unwrap(), bytes);

    let removed = library
        .with_connection(|connection| attachment_repository::delete(connection, &attachment_id))
        .unwrap();
    assert_eq!(removed.as_deref(), Some(relative.as_str()));
    let gone = library
        .with_connection(|connection| attachment_repository::find_for_read(connection, &attachment_id))
        .unwrap();
    assert!(gone.is_none());

    std::fs::remove_dir_all(&root).ok();
}

/// A07：不支持的图片格式在写盘前就被拒绝。
#[test]
fn rejects_unsupported_image_format() {
    // BMP 文件头，足以让 guess_format 认出格式。
    let bmp = [b'B', b'M', 0, 0, 0, 0, 0, 0, 0, 0, 54, 0, 0, 0];
    assert!(matches!(
        attachments::inspect_image(&bmp),
        Err(AppError::UnsupportedImage(_))
    ));
}

fn sample_png(width: u32, height: u32) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(width, height, image::Rgba([120, 90, 60, 255]));
    let mut buffer = Vec::new();
    image::codecs::png::PngEncoder::new(&mut buffer)
        .write_image(
            image.as_bytes(),
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();
    buffer
}

