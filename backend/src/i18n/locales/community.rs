//! Community & Help errors. Both languages live next to each other.

use super::pair;
use crate::i18n::Lang;

pub enum CommunityError {
    QuestionAndDetailsRequired,
    RequestAlreadyClosed,
    ReplyContentRequired,
    LanguageInvalid,
    TitleAndContentRequired,
    GuideTitleRequired,
    GuideAudienceInvalid,
    HelpAttachmentInvalid,
    GuideSlugRequired,
    GuideSlugExists,
    GuideLinkInvalid,
    GuideStepInvalid,
    OnlyLinkOwnRides,
    AmountRangeInvalid,
    PayeeNameAndPhoneRequired,
    ReceiptRequired,
    OnlyReturnedReimbCanBeResubmitted,
    ReimbAlreadyReviewed,
    CannotReviewOwnRequest,
    OnlyApprovedReimbCanBePaid,
    CannotPayOwnRequest,
    CannotPayOwnApproval,
    ReimbursementAlreadyClaimed,
    EventRequiredForReimbursement,
    EventReimbursementNeedsSignup,
    PayeeVerificationRequired,
    ReimbursementProcessed,
    ReasonRequired,
    MonthFormatInvalid,
    BatchEmpty,
    BatchAlreadyPaid,
    VisitorReimbNeedsDriver,
    VisitorReimbAlreadyLive,
    VisitorHasReimbursement,
    AmountMustBePositive,
}

pub fn render(lang: Lang, key: CommunityError) -> String {
    match key {
        CommunityError::QuestionAndDetailsRequired => pair(
            lang,
            "请填写问题与说明",
            "Please enter your question and details",
        ),
        CommunityError::RequestAlreadyClosed => {
            pair(lang, "此求助已结案", "This request has already been closed")
        }
        CommunityError::ReplyContentRequired => pair(lang, "请填写内容", "Please enter your reply"),
        CommunityError::LanguageInvalid => {
            pair(lang, "语言只支持中文或英文", "Language must be zh or en")
        }
        CommunityError::TitleAndContentRequired => pair(
            lang,
            "请填写标题及内容",
            "Please enter both a title and content",
        ),
        CommunityError::GuideTitleRequired => pair(
            lang,
            "请至少填写一种语言的指南标题",
            "Enter the guide title in at least one language",
        ),
        CommunityError::GuideAudienceInvalid => pair(
            lang,
            "指南对象只支持会员或职员",
            "Guide audience must be member or staff",
        ),
        CommunityError::HelpAttachmentInvalid => pair(
            lang,
            "只能附上您自己上传的图片",
            "Attach a photo you uploaded yourself",
        ),
        CommunityError::GuideSlugRequired => pair(
            lang,
            "请填写有效的 slug（小写字母、数字、连字符）",
            "Enter a valid slug (lowercase letters, numbers, dashes)",
        ),
        CommunityError::GuideSlugExists => {
            pair(lang, "该 slug 已被使用", "That slug is already in use")
        }
        CommunityError::GuideLinkInvalid => pair(
            lang,
            "步骤链接必须是应用内路径（以 / 开头）",
            "A step link must be an in-app path starting with /",
        ),
        CommunityError::GuideStepInvalid => pair(
            lang,
            "每个步骤至少填写一种语言的标题；最多 30 个步骤",
            "Every step needs a title in at least one language; 30 steps max",
        ),
        CommunityError::OnlyLinkOwnRides => {
            pair(lang, "只能关联自己的接送", "You can only link your own rides")
        }
        CommunityError::AmountRangeInvalid => pair(
            lang,
            "金额需介于 RM0.01 – RM500",
            "Amount must be between RM0.01 and RM500",
        ),
        CommunityError::PayeeNameAndPhoneRequired => pair(
            lang,
            "请填写收款人姓名及手机号（TNG / DuitNow）",
            "Please enter the payee name and phone (TNG / DuitNow)",
        ),
        CommunityError::ReceiptRequired => pair(lang, "请上传收据", "Please upload a receipt"),
        CommunityError::OnlyReturnedReimbCanBeResubmitted => pair(
            lang,
            "只有被退回的申请可以补资料重新提交",
            "Only returned requests can be resubmitted",
        ),
        CommunityError::ReimbAlreadyReviewed => {
            pair(lang, "此申请已审核", "This request has already been reviewed")
        }
        CommunityError::CannotReviewOwnRequest => {
            pair(lang, "不能审核自己的申请", "You cannot review your own request")
        }
        CommunityError::OnlyApprovedReimbCanBePaid => pair(
            lang,
            "只有已批准的申请可以付款",
            "Only approved requests can be paid",
        ),
        CommunityError::CannotPayOwnApproval => pair(
            lang,
            "批准与付款需由不同的人处理：此申请由您批准，请交给另一位财务同工付款",
            "Approving and paying need two people: you approved this request, so another finance member must pay it",
        ),
        CommunityError::VisitorReimbNeedsDriver => pair(
            lang,
            "请先为这位访客选好所乘的车：特别报销付给该司机",
            "Choose the car this visitor rode in first: the special reimbursement pays that driver",
        ),
        CommunityError::VisitorReimbAlreadyLive => pair(
            lang,
            "这位访客已有一笔特别报销在处理中或已批准；只有被拒绝后才能再申请",
            "This visitor already has a special reimbursement in progress or approved; a new one is only possible after it is rejected",
        ),
        CommunityError::AmountMustBePositive => pair(
            lang,
            "金额需大于 RM0",
            "The amount must be more than RM0",
        ),
        CommunityError::VisitorHasReimbursement => pair(
            lang,
            "这位访客有特别报销记录，不能删除",
            "This visitor has a special reimbursement on record and can't be removed",
        ),
        CommunityError::ReimbursementAlreadyClaimed => pair(
            lang,
            "此收据或此趟接送已有报销申请，请勿重复申请",
            "This receipt or ride already has a reimbursement request; please don't claim it twice",
        ),
        CommunityError::EventRequiredForReimbursement => pair(
            lang,
            "Grab 报销需关联活动",
            "Grab reimbursement requires an event",
        ),
        CommunityError::EventReimbursementNeedsSignup => pair(
            lang,
            "请先报名参加此活动，才能申请 Grab 报销",
            "You must be registered for the event to claim Grab reimbursement",
        ),
        CommunityError::CannotPayOwnRequest => {
            pair(lang, "不能为自己的申请付款", "You cannot pay your own request")
        }
        CommunityError::PayeeVerificationRequired => pair(
            lang,
            "请先核对收款人姓名、手机号及 DuitNow 收款去向",
            "Please verify the payee name, phone and DuitNow destination first",
        ),
        CommunityError::ReimbursementProcessed => {
            pair(lang, "此报销已处理", "This claim has already been handled")
        }
        CommunityError::ReasonRequired => pair(lang, "请填写原因", "Please provide a reason"),
        CommunityError::MonthFormatInvalid => pair(
            lang,
            "月份格式应为 YYYY-MM",
            "Month must be in YYYY-MM format",
        ),
        CommunityError::BatchEmpty => pair(
            lang,
            "没有符合条件可加入批次的已批准报销",
            "No eligible approved reimbursements available for this batch",
        ),
        CommunityError::BatchAlreadyPaid => pair(
            lang,
            "此付款批次已完成付款",
            "This payout batch has already been paid",
        ),
    }
}
