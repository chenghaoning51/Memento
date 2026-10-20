use std::collections::HashMap;
use std::path::PathBuf;

use image::{EncodableLayout, ImageEncoder};
use rusqlite::Connection;
use uuid::Uuid;

use crate::attachments::{self, repository as attachment_repository, Attachment};
use crate::backup;
use crate::db::migrations;
use crate::entries::{repository, EntryDraft, SearchQuery};
use crate::letters::{self, repository as letter_repository, LetterDraft};
use crate::error::AppError;
use crate::export::{self, ExportEntry};
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

/// A16：导出把正文降级成 Markdown，图片用相对路径，重名不覆盖。
#[test]
fn export_writes_markdown_and_copies_images() {
    let root = temp_library_root("export");
    let target = root.join("out");
    std::fs::create_dir_all(root.join("attachments/entry-1")).unwrap();
    std::fs::write(root.join("attachments/entry-1/pic.png"), sample_png(20, 10)).unwrap();

    let content = r#"{"version":2,"type":"doc","content":[
        {"type":"heading","attrs":{"level":2},"content":[{"type":"text","text":"下午"}]},
        {"type":"paragraph","content":[
            {"type":"text","text":"窗外的"},
            {"type":"text","marks":[{"type":"bold"}],"text":"梧桐叶"},
            {"type":"text","text":"被打湿了。"}
        ]},
        {"type":"blockquote","content":[{"type":"paragraph","content":[{"type":"text","text":"雨还在下。"}]}]},
        {"type":"horizontalRule"},
        {"type":"bulletList","content":[
            {"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"带伞"}]}]},
            {"type":"listItem","content":[{"type":"paragraph","content":[
                {"type":"text","marks":[{"type":"strike"}],"text":"等雨停"}
            ]}]}
        ]},
        {"type":"orderedList","attrs":{"start":1},"content":[
            {"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"买咖啡"}]}]},
            {"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"写日记"}]}]}
        ]},
        {"type":"paragraph","content":[{"type":"image","attrs":{"alt":"窗外的树","attachmentId":"att-1"}}]}
    ]}"#;

    let entries = vec![ExportEntry {
        entry_date: "2026-10-17".to_string(),
        title: "雨天/散步".to_string(),
        content_json: content.to_string(),
        excerpt: "窗外的梧桐叶被打湿了。".to_string(),
    }];
    let mut attachments = HashMap::new();
    attachments.insert("att-1".to_string(), "attachments/entry-1/pic.png".to_string());

    let first = export::write_entries(&target, &root, &entries, &attachments).unwrap();
    assert_eq!(first.entry_count, 1);
    assert_eq!(first.image_count, 1);

    let markdown = std::fs::read_to_string(target.join("2026-10-17-雨天 散步.md")).unwrap();
    assert!(markdown.starts_with("# 雨天/散步\n\n"));
    assert!(markdown.contains("## 下午"));
    assert!(markdown.contains("窗外的**梧桐叶**被打湿了。"));
    assert!(markdown.contains("> 雨还在下。"));
    assert!(markdown.contains("---"));
    assert!(markdown.contains("- 带伞"));
    assert!(markdown.contains("- ~~等雨停~~"));
    assert!(markdown.contains("1. 买咖啡"));
    assert!(markdown.contains("2. 写日记"));
    assert!(markdown.contains("![窗外的树](images/pic.png)"));
    assert!(target.join("images/pic.png").exists());

    // 同一个目录再导出一次：文件名追加序号，不覆盖已有文件。
    let second = export::write_entries(&target, &root, &entries, &attachments).unwrap();
    assert_eq!(second.image_count, 0);
    assert!(target.join("2026-10-17-雨天 散步 (2).md").exists());

    std::fs::remove_dir_all(&root).ok();
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

/// A17、A18：备份可校验、可恢复；损坏的备份在恢复前被拒绝。
#[test]
fn backup_restores_the_whole_space_and_rejects_damage() {
    let root = temp_library_root("backup");
    let backup_parent = temp_library_root("backup-out");
    let library = Library::open(root.clone()).unwrap();

    write_entry(&library, "entry-1", "2026-10-17", "备份前的日记", "今天下雨了。", CREATED_AT);
    library
        .with_connection(|connection| {
            tag_repository::add(connection, "entry-1", "备份")?;
            letter_repository::insert(
                connection,
                "letter-1",
                &LetterDraft {
                    title: "三个月后".to_string(),
                    content_json: EMPTY_CONTENT_JSON.to_string(),
                    content_text: "写给自己。".to_string(),
                    unlock_at_utc: "2027-01-01T00:00:00Z".to_string(),
                    unlock_timezone: "Asia/Shanghai".to_string(),
                },
                letters::STATUS_LOCKED,
                CREATED_AT,
            )
        })
        .unwrap();

    let summary = library
        .with_connection(|connection| {
            backup::create(connection, &root, &backup_parent, CREATED_AT, "0.1.0")
        })
        .unwrap();
    let directory = PathBuf::from(&summary.directory);
    assert!(directory.join("manifest.json").exists());

    // 预检读出内容规模；备份之后又写了一篇，恢复应该把它换掉。
    let info = backup::inspect(&directory).unwrap();
    assert_eq!(info.entry_count, 1);
    assert_eq!(info.letter_count, 1);
    write_entry(&library, "entry-2", "2026-10-18", "备份之后", "后来写的。", CREATED_AT);

    let restored = library
        .with_connection(|connection| {
            backup::restore(connection, &root, &directory, "2026-10-18T09:00:00Z", "0.1.0")
        })
        .unwrap();
    assert_eq!(restored.entry_count, 1);
    assert_eq!(restored.letter_count, 1);
    assert!(PathBuf::from(&restored.pre_restore_backup).exists());

    let entries = library
        .with_connection(|connection| repository::list_by_date(connection, "2026-10-18"))
        .unwrap();
    assert!(entries.is_empty());
    let tags = library
        .with_connection(|connection| tag_repository::list_for_entry(connection, "entry-1"))
        .unwrap();
    assert_eq!(tags.len(), 1);
    let letter = library
        .with_connection(|connection| letter_repository::find(connection, "letter-1"))
        .unwrap();
    assert!(letter.is_some());

    // 改坏的备份在恢复前被拒绝，当前空间不受影响：校验值不符与文件缺失都算坏。
    std::fs::write(directory.join("memento.db"), b"broken").unwrap();
    assert!(matches!(backup::inspect(&directory), Err(AppError::InvalidBackup(_))));
    std::fs::remove_file(directory.join("memento.db")).unwrap();
    assert!(matches!(backup::inspect(&directory), Err(AppError::InvalidBackup(_))));
    assert!(matches!(
        library.with_connection(|connection| {
            backup::restore(connection, &root, &directory, "2026-10-18T10:00:00Z", "0.1.0")
        }),
        Err(AppError::InvalidBackup(_))
    ));
    let entries = library
        .with_connection(|connection| repository::list_by_date(connection, "2026-10-17"))
        .unwrap();
    assert_eq!(entries.len(), 1);

    std::fs::remove_dir_all(&root).ok();
    std::fs::remove_dir_all(&backup_parent).ok();
}

/// 永久删除：只接受回收站里的日记，正文、附件行与附件文件、孤儿标签一起清掉。
#[test]
fn purge_removes_entry_attachments_and_orphan_tags() {
    let root = temp_library_root("purge");
    let library = Library::open(root.clone()).unwrap();
    let entry_id = Uuid::new_v4().to_string();
    let attachment_id = Uuid::new_v4().to_string();

    library
        .with_connection(|connection| {
            repository::create(connection, &entry_id, "2026-10-17", EMPTY_CONTENT_JSON, CREATED_AT)?;
            tag_repository::add(connection, &entry_id, "工作")?;
            tag_repository::add(connection, &entry_id, "旅行")?;

            let relative = attachments::original_path(&entry_id, &attachment_id, "png");
            attachments::write_file(library.root(), &relative, &sample_png(2, 2))?;
            let thumbnail = attachments::thumbnail_path(&attachment_id);
            attachments::write_file(library.root(), &thumbnail, &sample_png(1, 1))?;
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

    // 正常日记不能直接永久删除，必须先经过回收站。
    assert!(matches!(
        library.with_connection(|connection| repository::purge(connection, &entry_id)),
        Err(AppError::EntryNotInTrash)
    ));

    library
        .with_connection(|connection| {
            repository::set_deleted_at(connection, &entry_id, Some("2026-10-17T09:00:00Z"))
        })
        .unwrap();

    let paths = library
        .with_connection(|connection| repository::purge(connection, &entry_id))
        .unwrap();
    assert_eq!(paths.len(), 1);

    // 正文、附件行与标签关系都已消失；不再被引用的标签也一起清掉。
    assert!(matches!(
        library.with_connection(|connection| repository::require(connection, &entry_id)),
        Err(AppError::MissingEntry(_))
    ));
    assert!(library
        .with_connection(|connection| attachment_repository::find_for_read(connection, &attachment_id))
        .unwrap()
        .is_none());
    assert!(library
        .with_connection(|connection| tag_repository::list_for_entry(connection, &entry_id))
        .unwrap()
        .is_empty());
    assert!(library
        .with_connection(tag_repository::list_all)
        .unwrap()
        .is_empty());
    assert!(library
        .with_connection(repository::list_deleted)
        .unwrap()
        .is_empty());

    // 清空回收站：多篇一次删掉，返回篇数。
    for index in 0..3 {
        let id = format!("purge-{index}");
        library
            .with_connection(|connection| {
                repository::create(connection, &id, "2026-10-18", EMPTY_CONTENT_JSON, CREATED_AT)?;
                repository::set_deleted_at(connection, &id, Some("2026-10-18T09:00:00Z"))
            })
            .unwrap();
    }
    library
        .with_connection(|connection| {
            repository::create(connection, "kept", "2026-10-18", EMPTY_CONTENT_JSON, CREATED_AT)
        })
        .unwrap();

    let (count, _) = library
        .with_connection(repository::purge_deleted)
        .unwrap();
    assert_eq!(count, 3);
    let remaining = library
        .with_connection(|connection| repository::list_by_date(connection, "2026-10-18"))
        .unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, "kept");

    std::fs::remove_dir_all(&root).ok();
}

/// 选择性导出：只导出选中的日记；空列表表示全部。
#[test]
fn export_can_target_selected_entries() {
    let root = temp_library_root("export-pick");
    let library = Library::open(root.clone()).unwrap();

    for (id, date, title) in [
        ("keep-1", "2026-10-01", "留下的第一篇"),
        ("skip", "2026-10-02", "不该导出的"),
        ("keep-2", "2026-10-03", "留下的第二篇"),
    ] {
        write_entry(&library, id, date, title, "正文。", CREATED_AT);
    }

    let all = library
        .with_connection(|connection| export::repository::list_for_export(connection, &[]))
        .unwrap();
    assert_eq!(all.len(), 3);

    let picked = library
        .with_connection(|connection| {
            export::repository::list_for_export(
                connection,
                &["keep-1".to_string(), "keep-2".to_string()],
            )
        })
        .unwrap();
    assert_eq!(picked.len(), 2);
    assert_eq!(picked[0].title, "留下的第一篇");
    assert_eq!(picked[1].title, "留下的第二篇");

    // 不存在的 ID 不会带出多余结果。
    let missing = library
        .with_connection(|connection| {
            export::repository::list_for_export(connection, &["nope".to_string()])
        })
        .unwrap();
    assert!(missing.is_empty());

    std::fs::remove_dir_all(&root).ok();
}

/// 自动备份只保留最近几份，用户自己的备份目录不受影响。
#[test]
fn prune_keeps_only_recent_auto_backups() {
    let root = temp_library_root("prune");
    let backups = root.join("backups");
    std::fs::create_dir_all(&backups).unwrap();

    // 造 7 份自动备份，外加一个不是备份的目录。
    for stamp in [
        "20260101000000",
        "20260102000000",
        "20260103000000",
        "20260104000000",
        "20260105000000",
        "20260106000000",
        "20260107000000",
    ] {
        std::fs::create_dir_all(backups.join(format!("memento-backup-{stamp}"))).unwrap();
    }
    std::fs::create_dir_all(backups.join("my-own-folder")).unwrap();

    let removed = backup::prune_auto_backups(&backups, 5).unwrap();
    assert_eq!(removed.len(), 2);

    let mut left: Vec<String> = std::fs::read_dir(&backups)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    left.sort();
    assert_eq!(left.len(), 6);
    assert!(left.contains(&"my-own-folder".to_string()));
    assert!(!left.contains(&"memento-backup-20260101000000".to_string()));
    assert!(!left.contains(&"memento-backup-20260102000000".to_string()));
    assert!(left.contains(&"memento-backup-20260107000000".to_string()));

    std::fs::remove_dir_all(&root).ok();
}
