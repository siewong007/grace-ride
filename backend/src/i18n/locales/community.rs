//! Community & Help errors. Both languages live next to each other.

use super::pair;
use crate::i18n::{keys::ErrorKey, Lang};

pub fn render(lang: Lang, key: &ErrorKey) -> String {
    match key {
        ErrorKey::QuestionAndDetailsRequired => pair(
            lang,
            "请填写问题与说明",
            "Please enter your question and details",
        ),
        ErrorKey::RequestAlreadyClosed => {
            pair(lang, "此求助已结案", "This request has already been closed")
        }
        ErrorKey::ReplyContentRequired => pair(lang, "请填写内容", "Please enter your reply"),
        ErrorKey::LanguageInvalid => {
            pair(lang, "语言只支持中文或英文", "Language must be zh or en")
        }
        ErrorKey::TitleAndContentRequired => pair(
            lang,
            "请填写标题及内容",
            "Please enter both a title and content",
        ),
        ErrorKey::GuideTitleRequired => pair(
            lang,
            "请至少填写一种语言的指南标题",
            "Enter the guide title in at least one language",
        ),
        ErrorKey::GuideAudienceInvalid => pair(
            lang,
            "指南对象只支持会员或职员",
            "Guide audience must be member or staff",
        ),
        ErrorKey::GuideSlugRequired => pair(
            lang,
            "请填写有效的 slug（小写字母、数字、连字符）",
            "Enter a valid slug (lowercase letters, numbers, dashes)",
        ),
        ErrorKey::GuideSlugExists => pair(lang, "该 slug 已被使用", "That slug is already in use"),
        ErrorKey::GuideLinkInvalid => pair(
            lang,
            "步骤链接必须是应用内路径（以 / 开头）",
            "A step link must be an in-app path starting with /",
        ),
        ErrorKey::GuideStepInvalid => pair(
            lang,
            "每个步骤至少填写一种语言的标题；最多 30 个步骤",
            "Every step needs a title in at least one language; 30 steps max",
        ),
        ErrorKey::OnlyLinkOwnRides => pair(
            lang,
            "只能关联自己的接送",
            "You can only link your own rides",
        ),
        ErrorKey::AmountRangeRM001to500 => pair(
            lang,
            "金额需介于 RM0.01 – RM500",
            "Amount must be between RM0.01 and RM500",
        ),
        ErrorKey::PayeeNameAndPhoneRequired => pair(
            lang,
            "请填写收款人姓名及手机号（TNG / DuitNow）",
            "Please enter the payee name and phone (TNG / DuitNow)",
        ),
        ErrorKey::ReceiptRequired => pair(lang, "请上传收据", "Please upload a receipt"),
        ErrorKey::ReimbursementOnlyPassenger => pair(
            lang,
            "只有乘客可以申请 Grab 报销",
            "Only passengers can claim Grab reimbursement",
        ),
        ErrorKey::ReimbursementOnlyNonPending => pair(
            lang,
            "行程尚未确认或已取消，不能报销",
            "Reimbursement is only for confirmed, non-cancelled rides",
        ),
        ErrorKey::OnlyReturnedReimbCanBeResubmitted => pair(
            lang,
            "只有被退回的申请可以补资料重新提交",
            "Only returned requests can be resubmitted",
        ),
        ErrorKey::ReimbAlreadyReviewed => pair(
            lang,
            "此申请已审核",
            "This request has already been reviewed",
        ),
        ErrorKey::CannotReviewOwnRequest => pair(
            lang,
            "不能审核自己的申请",
            "You cannot review your own request",
        ),
        ErrorKey::OnlyApprovedReimbCanBePaid => pair(
            lang,
            "只有已批准的申请可以付款",
            "Only approved requests can be paid",
        ),
        ErrorKey::CannotPayOwnRequest => pair(
            lang,
            "不能为自己的申请付款",
            "You cannot pay your own request",
        ),
        ErrorKey::PayeeVerificationRequired => pair(
            lang,
            "请先核对收款人姓名、手机号及 DuitNow 收款去向",
            "Please verify the payee name, phone and DuitNow destination first",
        ),
        ErrorKey::ReimbursementProcessed => {
            pair(lang, "此报销已处理", "This claim has already been handled")
        }
        ErrorKey::ReasonRequired => pair(lang, "请填写原因", "Please provide a reason"),
        ErrorKey::MonthFormatInvalid => pair(
            lang,
            "月份格式应为 YYYY-MM",
            "Month must be in YYYY-MM format",
        ),
        other => panic!("{} is not a community error", other.code()),
    }
}
