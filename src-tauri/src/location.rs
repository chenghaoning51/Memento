//! 数据目录位置：默认在应用数据目录下，也可以由用户在设置里改到别处。
//!
//! 选择记在应用配置目录的 `library.json` 里，和 Library 本身分开存，
//! 这样即使数据目录被移动或删掉，应用仍能找到用户当初的选择并给出明确错误。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

const CONFIG_FILE: &str = "library.json";

#[derive(Debug, Serialize, Deserialize)]
struct LibraryConfig {
    root: String,
}

fn config_path(config_dir: &Path) -> PathBuf {
    config_dir.join(CONFIG_FILE)
}

/// 读用户选过的数据目录；没有配置或配置读不出来时返回 None，回退到默认位置。
pub fn configured_root(config_dir: &Path) -> Option<PathBuf> {
    let bytes = std::fs::read(config_path(config_dir)).ok()?;
    let config: LibraryConfig = serde_json::from_slice(&bytes).ok()?;
    let root = PathBuf::from(config.root.trim());
    if root.as_os_str().is_empty() {
        return None;
    }
    Some(root)
}

/// 记下用户选的数据目录。空路径表示恢复默认，直接删掉配置文件。
pub fn set_configured_root(config_dir: &Path, root: Option<&Path>) -> AppResult<()> {
    let path = config_path(config_dir);
    match root {
        None => {
            std::fs::remove_file(&path).ok();
            Ok(())
        }
        Some(root) => {
            std::fs::create_dir_all(config_dir)?;
            let config = LibraryConfig {
                root: root.display().to_string(),
            };
            std::fs::write(&path, serde_json::to_vec_pretty(&config)?)?;
            Ok(())
        }
    }
}

/// 目标目录里是否已经有 Memento 的数据。用来区分「空目录可以接管」和「已有空间」。
pub fn looks_like_library(directory: &Path) -> bool {
    directory.join("memento.db").exists()
}

/// 把一个目录里的整个空间复制到另一个目录：数据库、附件、缩略图。
/// 目标目录必须不存在或为空，避免覆盖别人的数据。
pub fn copy_space(source: &Path, destination: &Path) -> AppResult<()> {
    if destination.exists() {
        let empty = std::fs::read_dir(destination)?.next().is_none();
        if !empty {
            return Err(AppError::LibraryNotEmpty(
                destination.display().to_string(),
            ));
        }
    }
    std::fs::create_dir_all(destination)?;

    for name in ["memento.db", "memento.db-wal", "memento.db-shm"] {
        let from = source.join(name);
        if from.exists() {
            std::fs::copy(&from, destination.join(name))?;
        }
    }
    for name in ["attachments", "thumbnails", "backups"] {
        copy_tree(&source.join(name), &destination.join(name))?;
    }
    Ok(())
}

fn copy_tree(source: &Path, target: &Path) -> AppResult<()> {
    if !source.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let destination = target.join(entry.file_name());
        if path.is_dir() {
            copy_tree(&path, &destination)?;
        } else {
            std::fs::copy(&path, &destination)?;
        }
    }
    Ok(())
}
