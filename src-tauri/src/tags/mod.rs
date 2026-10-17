pub mod repository;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    pub name: String,
}

/// 标签最多 32 个字符，按字符数而不是字节数计。
pub const MAX_TAG_CHARS: usize = 32;

/// 显示名：去掉首尾空白，内部连续空白折叠成一个空格，保留原有大小写。
pub fn clean(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 比较与去重用的名字：在显示名的基础上忽略大小写。
pub fn normalized(name: &str) -> String {
    clean(name).to_lowercase()
}
