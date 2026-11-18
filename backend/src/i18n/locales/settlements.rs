//! Settlements errors. Both languages live next to each other.

use super::pair;
use crate::i18n::Lang;

pub enum SettlementsError<'a> {
    SettlementNameRequired,
    AtLeastOneExpenseItem,
    ExpenseItemNegativeOrBlank,
    AmountCannotBeNegative,
    SubsidyCannotBeNegative,
    SubsidyExceedsTotal,
    SelectParticipants,
    DuplicateParticipant,
    InvalidSplitMode,
    MemberAmountRequired,
    CustomSplitMismatch { sum: &'a str, total: &'a str },
    OnlyDraftsCanBeDeleted,
    OnlyOrganiserCanEditBill,
    OnlyOrganiserCanPublish,
    OnlyOrganiserCanCloseBill,
    SettlementAlreadyPublished,
    RevisionReasonRequired,
    UnattendedMembers(i64),
    OpenPaymentsRemain(i64),
    PaidMembersCannotBeRemoved(i64),
    PayeeDetailsRequired,
    OnlyPayeeCanVerify,
    ProofOnlyForOwnShare,
    ProofNotNeededNow,
    UploadReceiptFirst,
    InvalidProofDocument,
    TxnRefRequired,
    DuplicateTxnOrProof,
    NeedMoreInfoReasonRequired,
    NotOnCostSplitList,
    BillClosedCannotEdit,
    OnlyOrganiserCanInitiateCostSplit,
}

pub fn render(lang: Lang, key: SettlementsError<'_>) -> String {
    match key {
        SettlementsError::SettlementNameRequired => {
            pair(lang, "请填写结算名称", "Please enter a settlement name")
        }
        SettlementsError::AtLeastOneExpenseItem => pair(
            lang,
            "请至少填写一项开销",
            "Please add at least one expense item",
        ),
        SettlementsError::ExpenseItemNegativeOrBlank => pair(
            lang,
            "开销项目不能为负数或空白",
            "Expense items cannot be negative or blank",
        ),
        SettlementsError::AmountCannotBeNegative => {
            pair(lang, "金额不能为负数", "Amount cannot be negative")
        }
        SettlementsError::SubsidyCannotBeNegative => {
            pair(lang, "补贴不能为负数", "Subsidy cannot be negative")
        }
        SettlementsError::SubsidyExceedsTotal => pair(
            lang,
            "补贴不能超过开销总额",
            "Subsidy cannot exceed the total expenses",
        ),
        SettlementsError::SelectParticipants => {
            pair(lang, "请选择分摊成员", "Please select participants")
        }
        SettlementsError::DuplicateParticipant => pair(
            lang,
            "参与名单有重复",
            "Duplicate members in the participant list",
        ),
        SettlementsError::InvalidSplitMode => pair(lang, "分摊方式无效", "Invalid split mode"),
        SettlementsError::MemberAmountRequired => pair(
            lang,
            "请为每位成员填写金额",
            "Please enter an amount for each member",
        ),
        SettlementsError::CustomSplitMismatch { sum, total } => match lang {
            Lang::Zh => format!("个别金额合计 {sum} 与应分摊总额 {total} 不一致"),
            Lang::En => {
                format!("Individual amounts total {sum} but must equal the shared total {total}")
            }
        },
        SettlementsError::OnlyDraftsCanBeDeleted => {
            pair(lang, "只有草稿可以删除", "Only drafts can be deleted")
        }
        SettlementsError::OnlyOrganiserCanEditBill => {
            pair(lang, "只有发起者可以修改", "Only the organiser can edit this")
        }
        SettlementsError::OnlyOrganiserCanPublish => {
            pair(lang, "只有发起者可以发布", "Only the organiser can publish")
        }
        SettlementsError::OnlyOrganiserCanCloseBill => pair(
            lang,
            "只有发起者可以关闭账单",
            "Only the organiser can close this bill",
        ),
        SettlementsError::SettlementAlreadyPublished => pair(
            lang,
            "此结算已发布",
            "This settlement has already been published",
        ),
        SettlementsError::RevisionReasonRequired => pair(
            lang,
            "发布后修改金额，请填写修改原因",
            "After publishing, a revision reason is required to change amounts",
        ),
        SettlementsError::UnattendedMembers(count) => match lang {
            Lang::Zh => {
                format!("有 {count} 位成员尚未记录为“已出席”，请先在活动页面核实出席。")
            }
            Lang::En => format!(
                "{count} member(s) are not recorded as \"attended\" yet; please verify attendance on the event page first."
            ),
        },
        SettlementsError::OpenPaymentsRemain(count) => match lang {
            Lang::Zh => format!("尚有 {count} 位成员未确认付款"),
            Lang::En => format!("{count} member(s) have not confirmed payment yet"),
        },
        SettlementsError::PaidMembersCannotBeRemoved(count) => match lang {
            Lang::Zh => format!(
                "有 {count} 位成员已付款或已提交付款凭证，不能从名单移除。请先与对方处理退款，再在付款记录中重设其状态。"
            ),
            Lang::En => format!(
                "{count} member(s) have already paid or sent proof of payment, so they can't be removed. Sort out a refund with them first, then reset their payment."
            ),
        },
        SettlementsError::PayeeDetailsRequired => pair(
            lang,
            "请填写收款资料（TNG 手机号 / DuitNow / 银行）",
            "Please fill in payee details (TNG number / DuitNow / bank)",
        ),
        SettlementsError::OnlyPayeeCanVerify => {
            pair(lang, "只有收款者可以核对", "Only the payee can verify")
        }
        SettlementsError::ProofOnlyForOwnShare => pair(
            lang,
            "只能为自己的费用上传凭证",
            "You can only submit proof for your own share",
        ),
        SettlementsError::ProofNotNeededNow => pair(
            lang,
            "此费用目前不需要上传凭证",
            "Proof upload is not needed for this share at the moment",
        ),
        SettlementsError::UploadReceiptFirst => {
            pair(lang, "请先上传收据", "Please upload the receipt first")
        }
        SettlementsError::InvalidProofDocument => {
            pair(lang, "凭证无效", "Invalid proof document")
        }
        SettlementsError::TxnRefRequired => pair(
            lang,
            "请填写交易参考编号",
            "Please enter the transaction reference number",
        ),
        SettlementsError::DuplicateTxnOrProof => pair(
            lang,
            "此交易编号或收据已被使用，请确认是否重复上传",
            "This transaction reference or receipt has already been used; please check for a duplicate upload",
        ),
        SettlementsError::NeedMoreInfoReasonRequired => pair(
            lang,
            "请说明需补充的资料",
            "Please describe what information is needed",
        ),
        SettlementsError::NotOnCostSplitList => pair(
            lang,
            "您不在此分摊名单中",
            "You are not on this cost split's participant list",
        ),
        SettlementsError::BillClosedCannotEdit => pair(
            lang,
            "账单已关闭，不能修改",
            "This bill is closed and can no longer be edited",
        ),
        SettlementsError::OnlyOrganiserCanInitiateCostSplit => pair(
            lang,
            "只有活动发起者可以发起费用分摊",
            "Only the event organiser can create a cost split",
        ),
    }
}
