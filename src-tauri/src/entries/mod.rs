pub mod repository;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub entry_date: String,
    pub title: String,
    pub content_json: String,
    pub content_text: String,
    pub excerpt: String,
    pub cover_attachment_id: Option<String>,
    pub created_at_utc: String,
    pub updated_at_utc: String,
    pub deleted_at_utc: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntrySummary {
    pub id: String,
    pub entry_date: String,
    pub title: String,
    pub excerpt: String,
    pub cover_attachment_id: Option<String>,
    pub created_at_utc: String,
    pub updated_at_utc: String,
}

/// 月历与时间线用的日期摘要：某天有几篇、封面用哪张图。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryDateSummary {
    pub entry_date: String,
    pub entry_count: i64,
    pub cover_attachment_id: Option<String>,
}

/// 回收站条目：正常列表之外再带上删除时间。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletedEntry {
    pub id: String,
    pub entry_date: String,
    pub excerpt: String,
    pub cover_attachment_id: Option<String>,
    pub created_at_utc: String,
    pub deleted_at_utc: String,
}

/// 搜索条件：关键词为空表示不限，标签取交集，日期区间含两端。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    pub text: String,
    pub tag_ids: Vec<String>,
    pub start_date: String,
    pub end_date: String,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryDraft {
    pub id: String,
    pub title: String,
    pub content_json: String,
    pub content_text: String,
    pub cover_attachment_id: Option<String>,
}
