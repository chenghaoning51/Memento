//! Markdown 导出：把日记正文与图片写到用户选定的目录，导出后不依赖应用即可阅读。

pub mod repository;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use crate::error::AppResult;

/// 导出结果：写了几篇日记、复制了几张图片。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSummary {
    pub directory: String,
    pub entry_count: usize,
    pub image_count: usize,
}

/// 一篇待导出的日记：正文是 v2 的 ProseMirror JSON。
pub struct ExportEntry {
    pub entry_date: String,
    pub title: String,
    pub content_json: String,
    pub excerpt: String,
}

/// 把日记写成 Markdown 文件，图片复制到 images/ 并用相对路径引用。
pub fn write_entries(
    target: &Path,
    root: &Path,
    entries: &[ExportEntry],
    attachments: &HashMap<String, String>,
) -> AppResult<ExportSummary> {
    std::fs::create_dir_all(target)?;
    let image_dir = target.join("images");

    let mut image_count = 0;
    let mut used_names: HashMap<String, usize> = HashMap::new();

    for entry in entries {
        for relative in collect_image_paths(entry, attachments) {
            let source = root.join(&relative);
            if !source.exists() {
                continue;
            }
            std::fs::create_dir_all(&image_dir)?;
            let destination = image_dir.join(file_name_of(&relative));
            if !destination.exists() {
                std::fs::copy(&source, &destination)?;
                image_count += 1;
            }
        }

        let stem = format!("{}-{}", entry.entry_date, display_title(entry));
        let path = unique_path(target, &safe_file_name(&stem), "md", &mut used_names);
        std::fs::write(path, render_entry(entry, attachments))?;
    }

    Ok(ExportSummary {
        directory: target.display().to_string(),
        entry_count: entries.len(),
        image_count,
    })
}

/// 标题为空时用摘要前十几个字，再不行就叫“无题”。
fn display_title(entry: &ExportEntry) -> String {
    if !entry.title.trim().is_empty() {
        return entry.title.trim().to_string();
    }
    let excerpt = entry.excerpt.trim();
    if excerpt.is_empty() {
        return "无题".to_string();
    }
    excerpt.chars().take(16).collect()
}

/// 目录里的同名文件不覆盖，按“ (2)”“ (3)”依次追加序号。
fn unique_path(
    directory: &Path,
    stem: &str,
    extension: &str,
    used: &mut HashMap<String, usize>,
) -> PathBuf {
    let mut index = *used.get(stem).unwrap_or(&0) + 1;
    loop {
        let name = if index == 1 {
            format!("{stem}.{extension}")
        } else {
            format!("{stem} ({index}).{extension}")
        };
        let path = directory.join(&name);
        if !path.exists() {
            used.insert(stem.to_string(), index);
            return path;
        }
        index += 1;
    }
}

/// Windows 不允许的字符换成空格，连续空白折叠，超长标题截断。
fn safe_file_name(stem: &str) -> String {
    let cleaned: String = stem
        .chars()
        .map(|character| match character {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            character if (character as u32) < 0x20 => ' ',
            character => character,
        })
        .collect();
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.trim().trim_end_matches('.').chars().take(60).collect()
}

fn file_name_of(relative: &str) -> String {
    relative.rsplit('/').next().unwrap_or("image.png").to_string()
}

/// 正文 JSON 转 Markdown：只处理编辑器实际会产出的节点，未知节点降级成纯文本。
fn render_entry(entry: &ExportEntry, attachments: &HashMap<String, String>) -> String {
    let document: Value = serde_json::from_str(&entry.content_json).unwrap_or(Value::Null);
    let body = render_node(&document, attachments);
    let mut markdown = String::new();
    if !entry.title.trim().is_empty() {
        markdown.push_str(&format!("# {}\n\n", entry.title.trim()));
    }
    markdown.push_str(body.trim_end());
    markdown.push('\n');
    markdown
}

fn render_node(node: &Value, attachments: &HashMap<String, String>) -> String {
    let kind = node.get("type").and_then(Value::as_str).unwrap_or_default();
    match kind {
        "doc" => render_children(node, attachments),
        "paragraph" => format!("{}\n\n", render_children(node, attachments)),
        "heading" => {
            let level = node
                .get("attrs")
                .and_then(|attrs| attrs.get("level"))
                .and_then(Value::as_u64)
                .unwrap_or(1)
                .clamp(1, 6);
            format!(
                "{} {}\n\n",
                "#".repeat(level as usize),
                render_children(node, attachments).trim()
            )
        }
        "blockquote" => {
            let inner = render_children(node, attachments);
            let quoted = inner
                .trim_end()
                .split('\n')
                .map(|line| if line.is_empty() { ">".to_string() } else { format!("> {line}") })
                .collect::<Vec<_>>()
                .join("\n");
            format!("{quoted}\n\n")
        }
        "horizontalRule" => "---\n\n".to_string(),
        "hardBreak" => "  \n".to_string(),
        "bulletList" => render_list(node, attachments, false),
        "orderedList" => render_list(node, attachments, true),
        "text" => render_text(node),
        "image" => render_image(node, attachments),
        _ => render_children(node, attachments),
    }
}

/// 列表：每项缩进两格；子列表跟着父项内容一起缩进，嵌套层级才看得出来。
fn render_list(node: &Value, attachments: &HashMap<String, String>, ordered: bool) -> String {
    let start = node
        .get("attrs")
        .and_then(|attrs| attrs.get("start"))
        .and_then(Value::as_u64)
        .unwrap_or(1);

    let items = node
        .get("content")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    let marker = if ordered {
                        format!("{}. ", start + index as u64)
                    } else {
                        "- ".to_string()
                    };
                    let body = render_children(item, attachments);
                    let body = body.trim_end_matches('\n');
                    let indented = body.replace('\n', "\n  ");
                    format!("{marker}{indented}\n")
                })
                .collect::<String>()
        })
        .unwrap_or_default();

    format!("{items}\n")
}

fn render_children(node: &Value, attachments: &HashMap<String, String>) -> String {
    node.get("content")
        .and_then(Value::as_array)
        .map(|children| {
            children
                .iter()
                .map(|child| render_node(child, attachments))
                .collect::<String>()
        })
        .unwrap_or_default()
}

/// 行内格式：加粗、斜体、删除线、行内代码与链接。
fn render_text(node: &Value) -> String {
    let mut text = node.get("text").and_then(Value::as_str).unwrap_or_default().to_string();
    let marks = node.get("marks").and_then(Value::as_array);
    if let Some(marks) = marks {
        for mark in marks {
            match mark.get("type").and_then(Value::as_str).unwrap_or_default() {
                "bold" => text = format!("**{text}**"),
                "italic" => text = format!("*{text}*"),
                "strike" => text = format!("~~{text}~~"),
                "code" => text = format!("`{text}`"),
                "link" => {
                    let href = mark
                        .get("attrs")
                        .and_then(|attrs| attrs.get("href"))
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    if !href.is_empty() {
                        text = format!("[{text}]({href})");
                    }
                }
                _ => {}
            }
        }
    }
    text
}

/// 图片：只保留相对路径，文件名与复制到 images/ 的名字一致。
fn render_image(node: &Value, attachments: &HashMap<String, String>) -> String {
    let alt = node
        .get("attrs")
        .and_then(|attrs| attrs.get("alt"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let relative = node
        .get("attrs")
        .and_then(|attrs| attrs.get("attachmentId"))
        .and_then(Value::as_str)
        .and_then(|id| attachments.get(id));
    match relative {
        Some(relative) => format!("![{alt}](images/{})\n\n", file_name_of(relative)),
        None => String::new(),
    }
}

/// 正文里引用的附件相对路径，按出现顺序去重。
fn collect_image_paths(entry: &ExportEntry, attachments: &HashMap<String, String>) -> Vec<String> {
    let document: Value = serde_json::from_str(&entry.content_json).unwrap_or(Value::Null);
    let mut paths: Vec<String> = Vec::new();
    walk_images(&document, &mut |id| {
        if let Some(relative) = attachments.get(id) {
            if !paths.iter().any(|existing| existing == relative) {
                paths.push(relative.clone());
            }
        }
    });
    paths
}

fn walk_images(node: &Value, visit: &mut impl FnMut(&str)) {
    if node.get("type").and_then(Value::as_str) == Some("image") {
        if let Some(id) = node
            .get("attrs")
            .and_then(|attrs| attrs.get("attachmentId"))
            .and_then(Value::as_str)
        {
            visit(id);
        }
    }
    if let Some(children) = node.get("content").and_then(Value::as_array) {
        for child in children {
            walk_images(child, visit);
        }
    }
}
