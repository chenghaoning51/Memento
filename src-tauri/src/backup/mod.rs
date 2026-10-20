//! 完整空间备份与恢复：数据库快照、附件、缩略图与 manifest。
//!
//! 备份是一个目录：`memento.db`、`attachments/`、`thumbnails/` 和 `manifest.json`。
//! `manifest.json` 记录格式版本、模式版本、文件清单与每个文件的 SHA-256。

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::db::migrations;
use crate::error::{AppError, AppResult};

/// 备份目录格式版本。
const FORMAT_VERSION: i64 = 1;
/// 恢复前自动备份保留的份数：更旧的自动删掉，避免磁盘无限增长。
const AUTO_BACKUPS_TO_KEEP: usize = 5;
const MANIFEST_FILE: &str = "manifest.json";
const DATABASE_FILE: &str = "memento.db";
const ATTACHMENT_DIR: &str = "attachments";
const THUMBNAIL_DIR: &str = "thumbnails";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Manifest {
    format: i64,
    app_version: String,
    schema_version: i64,
    created_at_utc: String,
    files: Vec<ManifestFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestFile {
    path: String,
    size: u64,
    sha256: String,
}

/// 备份完成后的摘要。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSummary {
    pub directory: String,
    pub file_count: usize,
    pub byte_size: u64,
}

/// 恢复前的预检结果：先让用户看清要恢复的是什么。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub directory: String,
    pub app_version: String,
    pub schema_version: i64,
    pub created_at_utc: String,
    pub entry_count: i64,
    pub letter_count: i64,
    pub file_count: usize,
    pub byte_size: u64,
}

/// 恢复完成后的摘要。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreSummary {
    pub entry_count: i64,
    pub letter_count: i64,
    pub pre_restore_backup: String,
}

/// 生成一个带时间戳的备份目录名，避免覆盖之前的备份。
fn backup_directory(parent: &Path, now_utc: &str) -> PathBuf {
    let stamp: String = now_utc.chars().filter(char::is_ascii_digit).take(14).collect();
    parent.join(format!("memento-backup-{stamp}"))
}

/// 备份整个空间：先对数据库做一致性快照，再复制附件与缩略图，最后写 manifest。
pub fn create(
    connection: &Connection,
    root: &Path,
    parent: &Path,
    now_utc: &str,
    app_version: &str,
) -> AppResult<BackupSummary> {
    let target = backup_directory(parent, now_utc);
    std::fs::create_dir_all(&target)?;

    // VACUUM INTO 由 SQLite 自己写出一致快照，不需要暂停写入。
    let snapshot = target.join(DATABASE_FILE);
    connection.execute("VACUUM INTO ?1", [snapshot.to_string_lossy().to_string()])?;

    copy_tree(&root.join(ATTACHMENT_DIR), &target.join(ATTACHMENT_DIR))?;
    copy_tree(&root.join(THUMBNAIL_DIR), &target.join(THUMBNAIL_DIR))?;

    let schema_version = migrations::schema_version(connection)?;
    let files = collect_files(&target)?;
    let byte_size = files.iter().map(|file| file.size).sum();
    let file_count = files.len();
    let manifest = Manifest {
        format: FORMAT_VERSION,
        app_version: app_version.to_string(),
        schema_version,
        created_at_utc: now_utc.to_string(),
        files,
    };
    // manifest 最后写：没有 manifest 的目录就是没写完的备份。
    std::fs::write(target.join(MANIFEST_FILE), serde_json::to_vec_pretty(&manifest)?)?;

    Ok(BackupSummary {
        directory: target.display().to_string(),
        file_count,
        byte_size,
    })
}

/// 只保留最新的若干份自动备份，返回删掉的目录。
///
/// 只处理 `backups/` 下名字符合 `memento-backup-<14 位时间戳>` 的目录，
/// 用户自己选的备份位置不会被碰。
pub fn prune_auto_backups(backups_dir: &Path, keep: usize) -> AppResult<Vec<String>> {
    if !backups_dir.exists() {
        return Ok(Vec::new());
    }

    let mut candidates: Vec<(String, PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(backups_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(stamp) = name.strip_prefix("memento-backup-") {
            if stamp.len() == 14 && stamp.chars().all(|character| character.is_ascii_digit()) {
                candidates.push((name, path));
            }
        }
    }

    // 名字里的时间戳可排序，按名字倒序就是新的在前。
    candidates.sort_by(|left, right| right.0.cmp(&left.0));

    let mut removed = Vec::new();
    for (_, path) in candidates.into_iter().skip(keep) {
        std::fs::remove_dir_all(&path)?;
        removed.push(path.display().to_string());
    }
    Ok(removed)
}

/// 读取并校验备份目录：manifest 版本、文件清单与校验值都要对得上。
pub fn inspect(directory: &Path) -> AppResult<BackupInfo> {
    let manifest = read_manifest(directory)?;
    verify_files(directory, &manifest)?;

    let snapshot = Connection::open(directory.join(DATABASE_FILE))?;
    let entry_count: i64 = snapshot.query_row(
        "SELECT COUNT(*) FROM entries WHERE deleted_at_utc IS NULL",
        [],
        |row| row.get(0),
    )?;
    let letter_count: i64 =
        snapshot.query_row("SELECT COUNT(*) FROM letters", [], |row| row.get(0))?;

    Ok(BackupInfo {
        directory: directory.display().to_string(),
        app_version: manifest.app_version,
        schema_version: manifest.schema_version,
        created_at_utc: manifest.created_at_utc,
        entry_count,
        letter_count,
        file_count: manifest.files.len(),
        byte_size: manifest.files.iter().map(|file| file.size).sum(),
    })
}

fn read_manifest(directory: &Path) -> AppResult<Manifest> {
    let path = directory.join(MANIFEST_FILE);
    if !path.exists() {
        return Err(AppError::InvalidBackup(
            "这个文件夹不是 Memento 的备份，换一个试试。".to_string(),
        ));
    }
    let manifest: Manifest = serde_json::from_slice(&std::fs::read(&path)?).map_err(|error| {
        eprintln!("读取备份说明失败：{error}");
        AppError::InvalidBackup("这份备份的说明文件读不出来，可能已经损坏。".to_string())
    })?;
    if manifest.format > FORMAT_VERSION {
        return Err(AppError::InvalidBackup(
            "这份备份来自更新版本的 Memento，请先升级再恢复。".to_string(),
        ));
    }
    Ok(manifest)
}

/// 逐个文件比对大小与 SHA-256：缺文件、被改动都要在恢复前拒绝。
fn verify_files(directory: &Path, manifest: &Manifest) -> AppResult<()> {
    for file in &manifest.files {
        let path = directory.join(&file.path);
        if !path.exists() {
            eprintln!("备份缺少文件：{}", file.path);
            return Err(AppError::InvalidBackup(
                "这份备份不完整，有文件缺失，恢复后可能看不到全部内容。换一份备份试试。".to_string(),
            ));
        }
        let bytes = std::fs::read(&path)?;
        if bytes.len() as u64 != file.size || sha256_hex(&bytes) != file.sha256 {
            eprintln!("备份文件校验不通过：{}", file.path);
            return Err(AppError::InvalidBackup(
                "这份备份里的文件被人改动过或已经损坏，为了安全没有恢复。".to_string(),
            ));
        }
    }
    Ok(())
}

/// 列出目录下所有文件的相对路径与校验值，路径统一用 / 分隔。
fn collect_files(root: &Path) -> AppResult<Vec<ManifestFile>> {
    let mut files = Vec::new();
    collect_into(root, root, &mut files)?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

fn collect_into(root: &Path, directory: &Path, files: &mut Vec<ManifestFile>) -> AppResult<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_into(root, &path, files)?;
            continue;
        }
        let bytes = std::fs::read(&path)?;
        let relative = path
            .strip_prefix(root)
            .expect("备份文件都在备份目录里")
            .to_string_lossy()
            .replace('\\', "/");
        files.push(ManifestFile {
            path: relative,
            size: bytes.len() as u64,
            sha256: sha256_hex(&bytes),
        });
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// 复制整个目录；源目录不存在时什么也不做。
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

/// 把备份装进当前空间。
///
/// 先把当前空间完整备份一份，再把备份的库复制到暂存目录并升级到当前模式版本，
/// 然后在一条事务里换掉所有行；附件与缩略图在事务成功后才换成备份里的那一份。
pub fn restore(
    connection: &Connection,
    root: &Path,
    directory: &Path,
    now_utc: &str,
    app_version: &str,
) -> AppResult<RestoreSummary> {
    let info = inspect(directory)?;
    let backups_dir = root.join("backups");
    let previous = create(connection, root, &backups_dir, now_utc, app_version)?;
    // 恢复前备份是自动产物，只保留最近几份。
    prune_auto_backups(&backups_dir, AUTO_BACKUPS_TO_KEEP)?;

    let staging = root.join(".restore");
    std::fs::remove_dir_all(&staging).ok();
    std::fs::create_dir_all(&staging)?;
    let staged_database = staging.join(DATABASE_FILE);
    std::fs::copy(directory.join(DATABASE_FILE), &staged_database)?;
    copy_tree(&directory.join(ATTACHMENT_DIR), &staging.join(ATTACHMENT_DIR))?;
    copy_tree(&directory.join(THUMBNAIL_DIR), &staging.join(THUMBNAIL_DIR))?;

    // 旧备份先升级到当前模式版本：换行时两边列数才一致。
    upgrade_snapshot(&staged_database, info.schema_version)?;

    replace_rows(connection, &staged_database)?;

    replace_directory(&staging.join(ATTACHMENT_DIR), &root.join(ATTACHMENT_DIR))?;
    replace_directory(&staging.join(THUMBNAIL_DIR), &root.join(THUMBNAIL_DIR))?;
    std::fs::remove_dir_all(&staging).ok();

    Ok(RestoreSummary {
        entry_count: info.entry_count,
        letter_count: info.letter_count,
        pre_restore_backup: previous.directory,
    })
}

fn upgrade_snapshot(path: &Path, previous_version: i64) -> AppResult<()> {
    let mut snapshot = Connection::open(path)?;
    migrations::migrate(&mut snapshot)?;
    if previous_version < 4 {
        crate::entries::repository::rebuild_search_text(&snapshot)?;
    }
    Ok(())
}

/// 在一条事务里把备份库的五张表整体换进来：中途失败就全部回滚。
fn replace_rows(connection: &Connection, snapshot: &Path) -> AppResult<()> {
    connection.execute("ATTACH DATABASE ?1 AS backup", [snapshot.to_string_lossy().to_string()])?;
    connection.pragma_update(None, "foreign_keys", "OFF")?;

    let result = (|| -> AppResult<()> {
        let transaction = connection.unchecked_transaction()?;
        transaction.execute_batch(
            "DELETE FROM entry_tags;
             DELETE FROM attachments;
             DELETE FROM tags;
             DELETE FROM entries;
             DELETE FROM letters;
             INSERT INTO entries SELECT * FROM backup.entries;
             INSERT INTO attachments SELECT * FROM backup.attachments;
             INSERT INTO tags SELECT * FROM backup.tags;
             INSERT INTO entry_tags SELECT * FROM backup.entry_tags;
             INSERT INTO letters SELECT * FROM backup.letters;",
        )?;
        transaction.commit()?;
        Ok(())
    })();

    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.execute("DETACH DATABASE backup", [])?;
    result
}

/// 用一个暂存目录替换当前目录；目标不存在时直接改名过去。
fn replace_directory(staged: &Path, live: &Path) -> AppResult<()> {
    if !staged.exists() {
        std::fs::remove_dir_all(live).ok();
        return Ok(());
    }
    std::fs::remove_dir_all(live).ok();
    std::fs::rename(staged, live)?;
    Ok(())
}