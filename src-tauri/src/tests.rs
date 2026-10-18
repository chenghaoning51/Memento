use std::path::PathBuf;

use image::{EncodableLayout, ImageEncoder};
use rusqlite::Connection;
use uuid::Uuid;

use crate::attachments::{self, repository as attachment_repository, Attachment};
use crate::db::migrations;
use crate::entries::{repository, EntryDraft, SearchQuery};
use crate::letters::{self, repository as letter_repository, LetterDraft};
use crate::error::AppError;
use crate::library::Library;
use crate::tags::{self, repository as tag_repository};

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
        let search_text = repository::search_text(&draft.title, &draft.content_text);
        let saved = library
            .with_connection(|connection| {
                repository::save(
                    connection,
                    &draft,
                    &excerpt,
                    &search_text,
                    "2026-10-17T08:01:00Z",
                )
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


/// A08：标签按规范化名去重，重复输入同一篇日记只留一个关联。
#[test]
fn tag_is_shared_across_entries_and_not_duplicated() {
    let root = temp_library_root("tags");
    let library = Library::open(root.clone()).unwrap();
    let first = Uuid::new_v4().to_string();
    let second = Uuid::new_v4().to_string();

    library
        .with_connection(|connection| {
            repository::create(connection, &first, "2026-10-17", EMPTY_CONTENT_JSON, CREATED_AT)?;
            repository::create(connection, &second, "2026-10-17", EMPTY_CONTENT_JSON, CREATED_AT)
        })
        .unwrap();

    let tags = library
        .with_connection(|connection| {
            tag_repository::add(connection, &first, &tags::clean(" 旅行   记录 "))?;
            tag_repository::add(connection, &first, "Travel")?;
            // 大小写不同、首尾带空白，规范化后与上一条相同，不应新增关联。
            tag_repository::add(connection, &first, "  travel  ")?;
            tag_repository::add(connection, &second, "travel")?;
            tag_repository::list_for_entry(connection, &first)
        })
        .unwrap();

    assert_eq!(
        tags.iter().map(|tag| tag.name.as_str()).collect::<Vec<_>>(),
        vec!["旅行 记录", "Travel"]
    );

    let tag_count: i64 = library
        .with_connection(|connection| {
            connection
                .query_row("SELECT count(*) FROM tags", [], |row| row.get(0))
                .map_err(AppError::from)
        })
        .unwrap();
    assert_eq!(tag_count, 2);

    let removed = library
        .with_connection(|connection| {
            tag_repository::remove(connection, &first, &tags[1].id)?;
            tag_repository::list_for_entry(connection, &first)
        })
        .unwrap();
    assert_eq!(removed.len(), 1);

    std::fs::remove_dir_all(&root).ok();
}

/// A09：软删除只写 deleted_at_utc，正文、附件和标签关系都不动；恢复后原样回来。
#[test]
fn deleted_entry_keeps_body_attachments_and_tags() {
    let root = temp_library_root("trash");
    let library = Library::open(root.clone()).unwrap();
    let entry_id = Uuid::new_v4().to_string();
    let attachment_id = Uuid::new_v4().to_string();

    library
        .with_connection(|connection| {
            repository::create(connection, &entry_id, "2026-10-17", EMPTY_CONTENT_JSON, CREATED_AT)?;
            tag_repository::add(connection, &entry_id, "工作")?;

            let relative = attachments::original_path(&entry_id, &attachment_id, "png");
            attachments::write_file(library.root(), &relative, &sample_png(2, 2))?;
            attachment_repository::insert(
                connection,
                &Attachment {
                    id: attachment_id.clone(),
                    entry_id: entry_id.clone(),
                    relative_path: relative,
                    mime_type: "image/png".to_string(),
                    file_size: 1,
                    sha256: String::new(),
                    width: 2,
                    height: 2,
                    created_at_utc: CREATED_AT.to_string(),
                },
            )
        })
        .unwrap();

    assert!(library
        .with_connection(|connection| repository::set_deleted_at(
            connection,
            &entry_id,
            Some("2026-10-17T09:00:00Z")
        ))
        .unwrap());

    let listed = library
        .with_connection(|connection| repository::list_by_date(connection, "2026-10-17"))
        .unwrap();
    assert!(listed.is_empty());

    let deleted = library.with_connection(repository::list_deleted).unwrap();
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted[0].id, entry_id);
    assert_eq!(deleted[0].deleted_at_utc, "2026-10-17T09:00:00Z");

    assert!(library
        .with_connection(|connection| repository::set_deleted_at(connection, &entry_id, None))
        .unwrap());

    let restored = library
        .with_connection(|connection| repository::require(connection, &entry_id))
        .unwrap();
    assert_eq!(restored.deleted_at_utc, None);

    let entry_tags = library
        .with_connection(|connection| tag_repository::list_for_entry(connection, &entry_id))
        .unwrap();
    assert_eq!(entry_tags.len(), 1);

    let attachment = library
        .with_connection(|connection| {
            attachment_repository::find_for_read(connection, &attachment_id)
        })
        .unwrap();
    assert!(attachment.is_some());

    std::fs::remove_dir_all(&root).ok();
}

/// A10：时间线按日期倒序分页，月历只统计当天未删除的日记并带上首图。
#[test]
fn timeline_and_calendar_read_entries_in_date_order() {
    let root = temp_library_root("timeline");
    let library = Library::open(root.clone()).unwrap();
    let cover_id = Uuid::new_v4().to_string();

    let dates = ["2026-10-15", "2026-10-17", "2026-10-15", "2026-10-16"];
    for (index, entry_date) in dates.iter().enumerate() {
        let id = Uuid::new_v4().to_string();
        let created = format!("2026-10-17T08:0{index}:00Z");
        library
            .with_connection(|connection| {
                repository::create(connection, &id, entry_date, EMPTY_CONTENT_JSON, &created)?;
                if index == 1 {
                    let draft = EntryDraft {
                        id: id.clone(),
                        title: String::new(),
                        content_json: EMPTY_CONTENT_JSON.to_string(),
                        content_text: String::new(),
                        cover_attachment_id: Some(cover_id.clone()),
                    };
                    repository::save(connection, &draft, "", "", &created)?;
                }
                Ok(())
            })
            .unwrap();
    }

    // 一篇被软删除，不应出现在时间线和月历里。
    let deleted_id = Uuid::new_v4().to_string();
    library
        .with_connection(|connection| {
            repository::create(
                connection,
                &deleted_id,
                "2026-10-16",
                EMPTY_CONTENT_JSON,
                "2026-10-17T08:10:00Z",
            )?;
            repository::set_deleted_at(connection, &deleted_id, Some("2026-10-17T09:00:00Z"))
        })
        .unwrap();

    let timeline = library
        .with_connection(|connection| repository::list_recent(connection, 3, 0))
        .unwrap();
    assert_eq!(
        timeline
            .iter()
            .map(|entry| entry.entry_date.as_str())
            .collect::<Vec<_>>(),
        vec!["2026-10-17", "2026-10-16", "2026-10-15"]
    );

    let next_page = library
        .with_connection(|connection| repository::list_recent(connection, 3, 3))
        .unwrap();
    assert_eq!(next_page.len(), 1);

    let days = library
        .with_connection(|connection| repository::list_dates(connection, "2026-10-01", "2026-10-31"))
        .unwrap();
    assert_eq!(
        days.iter()
            .map(|day| (day.entry_date.as_str(), day.entry_count))
            .collect::<Vec<_>>(),
        vec![("2026-10-15", 2), ("2026-10-16", 1), ("2026-10-17", 1)]
    );
    assert_eq!(
        days[2].cover_attachment_id.as_deref(),
        Some(cover_id.as_str())
    );

    std::fs::remove_dir_all(&root).ok();
}

/// A11：关键词、标签与日期范围可以组合筛选，软删除的日记不出现在结果里。
#[test]
fn search_matches_text_tags_and_dates_together() {
    let root = temp_library_root("search");
    let library = Library::open(root.clone()).unwrap();

    let rainy = Uuid::new_v4().to_string();
    let sunny = Uuid::new_v4().to_string();
    let older = Uuid::new_v4().to_string();
    let deleted = Uuid::new_v4().to_string();

    write_entry(
        &library,
        &rainy,
        "2026-10-17",
        "雨天",
        "今天下雨了。窗外的梧桐叶被打湿。",
        "2026-10-17T08:00:00Z",
    );
    write_entry(
        &library,
        &sunny,
        "2026-10-16",
        "",
        "下午天晴，晒被子。",
        "2026-10-16T08:00:00Z",
    );
    write_entry(
        &library,
        &older,
        "2026-09-20",
        "",
        "八月桂花香。",
        "2026-09-20T08:00:00Z",
    );
    write_entry(
        &library,
        &deleted,
        "2026-10-15",
        "",
        "下雨天删掉的记录。",
        "2026-10-15T08:00:00Z",
    );

    library
        .with_connection(|connection| {
            tag_repository::add(connection, &rainy, "天气")?;
            tag_repository::add(connection, &rainy, "散步")?;
            tag_repository::add(connection, &sunny, "天气")?;
            tag_repository::add(connection, &deleted, "天气")?;
            repository::set_deleted_at(connection, &deleted, Some("2026-10-17T09:00:00Z"))
        })
        .unwrap();

    let weather_tag = library
        .with_connection(tag_repository::list_all)
        .unwrap()
        .into_iter()
        .find(|tag| tag.name == "天气")
        .unwrap();

    let search = |query: SearchQuery| {
        library
            .with_connection(|connection| repository::search(connection, &query))
            .unwrap()
    };
    let query = |text: &str, tag_ids: Vec<String>, start: &str, end: &str| SearchQuery {
        text: text.to_string(),
        tag_ids,
        start_date: start.to_string(),
        end_date: end.to_string(),
        limit: 40,
        offset: 0,
    };

    // 只按关键词：软删除的日记不出现，大小写与空白不影响匹配。
    let hits = search(query("下雨", vec![], "", ""));
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, rainy);

    // 只按标签：两篇未删除的日记都命中。
    let hits = search(query("", vec![weather_tag.id.clone()], "", ""));
    assert_eq!(
        hits.iter().map(|entry| entry.id.as_str()).collect::<Vec<_>>(),
        vec![rainy.as_str(), sunny.as_str()]
    );

    // 标签取交集：同时有「天气」和「散步」的只有一篇。
    let walk_tag = library
        .with_connection(tag_repository::list_all)
        .unwrap()
        .into_iter()
        .find(|tag| tag.name == "散步")
        .unwrap();
    let hits = search(query(
        "",
        vec![weather_tag.id.clone(), walk_tag.id.clone()],
        "",
        "",
    ));
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, rainy);

    // 日期范围含两端：9 月的那篇落在区间外。
    let hits = search(query("", vec![], "2026-10-16", "2026-10-17"));
    assert_eq!(
        hits.iter().map(|entry| entry.id.as_str()).collect::<Vec<_>>(),
        vec![rainy.as_str(), sunny.as_str()]
    );

    // 组合条件：关键词 + 标签 + 日期。
    let hits = search(query(
        "下雨",
        vec![weather_tag.id.clone()],
        "2026-10-16",
        "2026-10-17",
    ));
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, rainy);
    assert_eq!(hits[0].excerpt, "今天下雨了。窗外的梧桐叶被打湿。");

    // 标题也参与匹配，且 % 不是通配符。
    assert_eq!(search(query("雨天", vec![], "", "")).len(), 1);
    assert_eq!(search(query("%", vec![], "", "")).len(), 0);

    // 分页：一次只取一篇时按日期倒序。
    let mut paged = query("", vec![], "", "");
    paged.limit = 1;
    let first = search(paged.clone());
    paged.offset = 1;
    let second = search(paged);
    assert_eq!(first[0].id, rainy);
    assert_eq!(second[0].id, sunny);

    std::fs::remove_dir_all(&root).ok();
}

fn write_entry(library: &Library, id: &str, entry_date: &str, title: &str, text: &str, created: &str) {
    let draft = EntryDraft {
        id: id.to_string(),
        title: title.to_string(),
        content_json: EMPTY_CONTENT_JSON.to_string(),
        content_text: text.to_string(),
        cover_attachment_id: None,
    };
    let excerpt = repository::build_excerpt(text);
    let search_text = repository::search_text(title, text);

    library
        .with_connection(|connection| {
            repository::create(connection, id, entry_date, EMPTY_CONTENT_JSON, created)?;
            repository::save(connection, &draft, &excerpt, &search_text, created)?;
            Ok(())
        })
        .unwrap();
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

/// A13、A14：未到期不可读，到期后能读原信并单独保存回应，打开时间不被覆盖。
#[test]
fn letter_stays_locked_until_unlock_time_and_keeps_reply() {
    let root = temp_library_root("letter");
    let id = Uuid::new_v4().to_string();
    let draft = LetterDraft {
        title: "给三个月后的自己".to_string(),
        content_json: EMPTY_CONTENT_JSON.to_string(),
        content_text: "现在是十月的夜里，窗外有风。".to_string(),
        unlock_at_utc: "2026-12-24T16:00:00.000Z".to_string(),
        unlock_timezone: "Asia/Shanghai".to_string(),
    };

    let library = Library::open(root.clone()).unwrap();
    library
        .with_connection(|connection| {
            letter_repository::insert(connection, &id, &draft, letters::STATUS_LOCKED, CREATED_AT)
        })
        .unwrap();

    // 没到期：状态停在 locked，开启时刻与时区都按写信时的样子存着。
    let letter = library
        .with_connection(|connection| letter_repository::find(connection, &id))
        .unwrap()
        .unwrap();
    assert_eq!(letter.status, letters::STATUS_LOCKED);
    assert_eq!(letter.unlock_at_utc, draft.unlock_at_utc);
    assert_eq!(letter.unlock_timezone, "Asia/Shanghai");
    assert_eq!(letters::status_at(&letter.unlock_at_utc, "2026-10-17T13:40:00Z"), letters::STATUS_LOCKED);

    // 到期：状态推进到 opened 并记下打开时间，原信正文不变。
    let opened_at = "2026-12-24T16:00:01Z";
    library
        .with_connection(|connection| letter_repository::mark_opened(connection, &id, opened_at))
        .unwrap();
    let opened = library
        .with_connection(|connection| letter_repository::find(connection, &id))
        .unwrap()
        .unwrap();
    assert_eq!(opened.status, letters::STATUS_OPENED);
    assert_eq!(opened.opened_at_utc.as_deref(), Some(opened_at));
    assert_eq!(opened.content_text, draft.content_text);

    // 回应独立保存：原信不动，再读一次回应还在，打开时间不被覆盖。
    library
        .with_connection(|connection| {
            letter_repository::save_reply(connection, &id, EMPTY_CONTENT_JSON, "三个月后再看，已经过去了。", "2026-12-25T01:00:00Z")
        })
        .unwrap();
    library
        .with_connection(|connection| letter_repository::mark_opened(connection, &id, "2026-12-26T01:00:00Z"))
        .unwrap();
    let replied = library
        .with_connection(|connection| letter_repository::find(connection, &id))
        .unwrap()
        .unwrap();
    assert_eq!(replied.content_text, draft.content_text);
    assert_eq!(replied.reply_text.as_deref(), Some("三个月后再看，已经过去了。"));
    assert_eq!(replied.opened_at_utc.as_deref(), Some(opened_at));

    std::fs::remove_dir_all(&root).ok();
}

/// 状态只看存下来的 UTC 时刻：跨午夜与夏令时都不重新换算。
#[test]
fn letter_status_follows_stored_unlock_moment() {
    // 2026-10-18 00:00（Asia/Shanghai）与 2026-11-01 00:00（America/New_York，夏令时结束当天）。
    let shanghai = "2026-10-16T16:00:00.000Z";
    assert_eq!(letters::status_at(shanghai, "2026-10-16T15:59:59Z"), letters::STATUS_LOCKED);
    assert_eq!(letters::status_at(shanghai, "2026-10-16T16:00:00Z"), letters::STATUS_UNLOCKED);

    let new_york = "2026-11-01T04:00:00.000Z";
    assert_eq!(letters::status_at(new_york, "2026-11-01T03:59:59Z"), letters::STATUS_LOCKED);
    assert_eq!(letters::status_at(new_york, "2026-11-01T04:00:00Z"), letters::STATUS_UNLOCKED);
}

