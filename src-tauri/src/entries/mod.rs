pub mod repository;

use serde::{Deserialize, Serialize};

/// 一篇日记的完整内容。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub entry_date: String,
    pub title: String,
    pub content_json: String,
    pub content_text: String,
    pub excerpt: String,
    pub created_at_utc: String,
    pub updated_at_utc: String,
    pub deleted_at_utc: Option<String>,
}

/// 列表里一行日记所需的最小字段。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntrySummary {
    pub id: String,
    pub entry_date: String,
    pub title: String,
    pub excerpt: String,
    pub created_at_utc: String,
    pub updated_at_utc: String,
}

/// 前端提交的一次保存。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryDraft {
    pub id: String,
    pub title: String,
    pub content_json: String,
    pub content_text: String,
}
