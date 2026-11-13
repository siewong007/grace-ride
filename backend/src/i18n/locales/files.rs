//! Files errors. Both languages live next to each other.

use super::pair;
use crate::i18n::{keys::ErrorKey, Lang};

pub fn render(lang: Lang, key: &ErrorKey) -> String {
    match key {
        ErrorKey::NoFileReceived => pair(lang, "没有收到文件", "No file received"),
        ErrorKey::FileTooLarge => pair(lang, "文件不能超过 8MB", "File cannot exceed 8MB"),
        ErrorKey::InvalidFileType => pair(
            lang,
            "只接受图片（JPG/PNG/WEBP/HEIC）或 PDF",
            "Only images (JPG/PNG/WEBP/HEIC) or PDF are accepted",
        ),
        ErrorKey::UploadFailed => pair(lang, "上传失败", "Upload failed"),
        ErrorKey::UploadQuotaReached => pair(
            lang,
            "今天上传的文件太多，请明天再试",
            "You have uploaded too many files today; please try again tomorrow",
        ),
        ErrorKey::InvalidReceipt => pair(lang, "收据无效", "Invalid receipt"),
        ErrorKey::InvalidPhoto => pair(lang, "照片无效", "Invalid photo"),
        ErrorKey::CannotViewFile => pair(lang, "您无法查看此文件", "You cannot view this file"),
        other => panic!("{} is not a files error", other.code()),
    }
}
