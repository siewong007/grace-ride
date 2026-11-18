//! Files errors. Both languages live next to each other.

use super::pair;
use crate::i18n::Lang;

pub enum FilesError {
    NoFileReceived,
    FileTooLarge,
    InvalidFileType,
    UploadFailed,
    UploadQuotaReached,
    InvalidReceipt,
    InvalidPhoto,
    CannotViewFile,
}

pub fn render(lang: Lang, key: FilesError) -> String {
    match key {
        FilesError::NoFileReceived => pair(lang, "没有收到文件", "No file received"),
        FilesError::FileTooLarge => pair(lang, "文件不能超过 8MB", "File cannot exceed 8MB"),
        FilesError::InvalidFileType => pair(
            lang,
            "只接受图片（JPG/PNG/WEBP/HEIC）或 PDF",
            "Only images (JPG/PNG/WEBP/HEIC) or PDF are accepted",
        ),
        FilesError::UploadFailed => pair(lang, "上传失败", "Upload failed"),
        FilesError::UploadQuotaReached => pair(
            lang,
            "今天上传的文件太多，请明天再试",
            "You have uploaded too many files today; please try again tomorrow",
        ),
        FilesError::InvalidReceipt => pair(lang, "收据无效", "Invalid receipt"),
        FilesError::InvalidPhoto => pair(lang, "照片无效", "Invalid photo"),
        FilesError::CannotViewFile => pair(lang, "您无法查看此文件", "You cannot view this file"),
    }
}
