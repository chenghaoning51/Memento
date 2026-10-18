pub mod repository;

use serde::{Deserialize, Serialize};

/// 锁定中：当前 UTC 早于开启时刻，正文不可读。
pub const STATUS_LOCKED: &str = "locked";
/// 已到期但还没读过。
pub const STATUS_UNLOCKED: &str = "unlocked";
/// 读过一次；打开时记下 opened_at_utc，之后一直保持。
pub const STATUS_OPENED: &str = "opened";

/// 一封信。锁定期间正文照样随记录返回，是否展示由前端按 unlocked 决定。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Letter {
    pub id: String,
    pub title: String,
    pub content_json: String,
    pub content_text: String,
    pub unlock_at_utc: String,
    pub unlock_timezone: String,
    pub status: String,
    pub reply_json: Option<String>,
    pub reply_text: Option<String>,
    pub opened_at_utc: Option<String>,
    pub created_at_utc: String,
    pub updated_at_utc: String,
}

/// 写信与改信共用的入参：开启时刻由前端按本地日期换算好再传。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LetterDraft {
    pub title: String,
    pub content_json: String,
    pub content_text: String,
    pub unlock_at_utc: String,
    pub unlock_timezone: String,
}

/// 状态只在写入与读取时按当前 UTC 推进：locked → unlocked → opened，不依赖后台定时器。
pub fn status_at(unlock_at_utc: &str, now_utc: &str) -> &'static str {
    if now_utc < unlock_at_utc {
        STATUS_LOCKED
    } else {
        STATUS_UNLOCKED
    }
}
