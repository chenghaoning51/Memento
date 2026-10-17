use serde::Serialize;

#[derive(Debug)]
pub enum AppError {
    Database(rusqlite::Error),
    Io(std::io::Error),
    UnsupportedSchema(i64),
    MissingEntry(String),
    UnsupportedImage(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Database(error) => write!(f, "数据库操作失败：{error}"),
            AppError::Io(error) => write!(f, "文件操作失败：{error}"),
            AppError::UnsupportedSchema(version) => write!(
                f,
                "数据库模式版本 {version} 高于当前应用支持的版本，请升级 Memento 后再打开"
            ),
            AppError::MissingEntry(id) => write!(f, "日记 {id} 已不存在，无法保存"),
            AppError::UnsupportedImage(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<rusqlite::Error> for AppError {
    fn from(error: rusqlite::Error) -> Self {
        AppError::Database(error)
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        AppError::Io(error)
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
