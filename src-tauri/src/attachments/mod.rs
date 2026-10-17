pub mod repository;

use std::io::Cursor;
use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageFormat, ImageReader};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};

/// 缩略图长边像素。时间线和小卡片只需要这个量级的预览。
const THUMBNAIL_EDGE: u32 = 320;
/// 缩略图 JPEG 质量。
const THUMBNAIL_QUALITY: u8 = 75;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: String,
    pub entry_id: String,
    pub relative_path: String,
    pub mime_type: String,
    pub file_size: i64,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub created_at_utc: String,
}

/// 导入前的判定结果：统一后缀和 MIME，后续路径和协议都用它。
pub struct ImageInfo {
    pub extension: &'static str,
    pub mime_type: &'static str,
    pub width: u32,
    pub height: u32,
}

/// 只接受 PNG 和 JPEG：两者浏览器能直接显示，也都能生成缩略图。
pub fn inspect_image(bytes: &[u8]) -> AppResult<ImageInfo> {
    let format = image::guess_format(bytes)
        .map_err(|_| AppError::UnsupportedImage("这个文件不是可以识别的图片。".into()))?;
    let (extension, mime_type) = match format {
        ImageFormat::Png => ("png", "image/png"),
        ImageFormat::Jpeg => ("jpg", "image/jpeg"),
        _ => return Err(AppError::UnsupportedImage("只支持 PNG 和 JPEG 图片。".into())),
    };

    let (width, height) = ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|_| AppError::UnsupportedImage("这张图片无法解析。".into()))?;

    Ok(ImageInfo {
        extension,
        mime_type,
        width,
        height,
    })
}

/// 缩略图：等比缩到长边 320 像素，存成 JPEG。
pub fn build_thumbnail(bytes: &[u8]) -> AppResult<Vec<u8>> {
    let image = image::load_from_memory(bytes)
        .map_err(|_| AppError::UnsupportedImage("这张图片无法解析。".into()))?;
    // JPEG 没有透明通道，带 alpha 的 PNG 必须先落成 RGB，否则编码器会因为缓冲区长度不符而失败。
    let thumbnail = image.thumbnail(THUMBNAIL_EDGE, THUMBNAIL_EDGE).to_rgb8();

    let mut buffer = Vec::new();
    JpegEncoder::new_with_quality(&mut buffer, THUMBNAIL_QUALITY)
        .encode(
            thumbnail.as_raw(),
            thumbnail.width(),
            thumbnail.height(),
            ExtendedColorType::Rgb8,
        )
        .map_err(|_| AppError::UnsupportedImage("无法生成缩略图。".into()))?;
    Ok(buffer)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// 原图相对路径：`attachments/<日记 ID>/<附件 ID>.<后缀>`，以 Library 根为基准。
pub fn original_path(entry_id: &str, attachment_id: &str, extension: &str) -> String {
    format!("attachments/{entry_id}/{attachment_id}.{extension}")
}

/// 缩略图相对路径：`thumbnails/<附件 ID>.jpg`，属于派生数据，丢失可重建。
pub fn thumbnail_path(attachment_id: &str) -> String {
    format!("thumbnails/{attachment_id}.jpg")
}

pub fn write_file(root: &Path, relative: &str, bytes: &[u8]) -> AppResult<()> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)?;
    Ok(())
}

pub fn remove_file(root: &Path, relative: &str) {
    std::fs::remove_file(root.join(relative)).ok();
}
