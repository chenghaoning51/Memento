use serde::Serialize;

#[derive(Debug)]
pub enum AppError {
    Database(rusqlite::Error),
    Io(std::io::Error),
    UnsupportedSchema(i64),
    MissingEntry(String),
    EntryNotInTrash,
    LibraryNotEmpty(String),
    UnsupportedImage(String),
    TagTooLong,
    MissingLetter(String),
    LetterLocked,
    LetterExpired,
    InvalidBackup(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // 底层错误原文只留给控制台，界面上只说明“发生了什么、可以怎么办”。
            AppError::Database(error) => {
                eprintln!("数据库操作失败：{error}");
                write!(f, "这次没能保存成功，请再试一次。如果一直这样，先在设置页做一次备份。")
            }
            AppError::Io(error) => {
                eprintln!("文件操作失败：{error}");
                write!(f, "读写文件时出错了，请再试一次。")
            }
            AppError::UnsupportedSchema(version) => {
                eprintln!("数据库模式版本 {version} 高于当前应用支持的版本");
                write!(f, "这个文件夹里的内容来自更新版本的 Memento，请先升级再打开。")
            }
            AppError::MissingEntry(_) => write!(f, "这篇日记已经不在了，可能已经在别处被删除。"),
            AppError::EntryNotInTrash => write!(f, "这篇日记不在回收站里，不能永久删除。"),
            AppError::LibraryNotEmpty(path) => {
                write!(f, "{path} 里已经有别的文件，请选一个空文件夹。")
            }
            AppError::UnsupportedImage(message) => write!(f, "{message}"),
            AppError::TagTooLong => write!(f, "标签最多 32 个字符。"),
            AppError::MissingLetter(_) => write!(f, "这封信已经不在了。"),
            AppError::LetterLocked => write!(f, "这封信还没有到开启时间。"),
            AppError::LetterExpired => write!(f, "这封信已经到期，原信不能再修改了。"),
            AppError::InvalidBackup(message) => write!(f, "{message}"),
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

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        eprintln!("备份清单读写失败：{error}");
        AppError::InvalidBackup("这份备份的说明文件读不出来，可能已经损坏。".into())
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
