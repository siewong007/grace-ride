//! Settlements errors. Both languages live next to each other.

use super::pair;
use crate::i18n::{keys::ErrorKey, Lang};

pub fn render(lang: Lang, key: &ErrorKey) -> String {
    match key {
        ErrorKey::SettlementNameRequired => pair(lang, "请填写结算名称", "Please enter a settlement name"),
        ErrorKey::AtLeastOneExpenseItem => pair(lang, "请至少填写一项开销", "Please add at least one expense item"),
        ErrorKey::ExpenseItemNegativeOrBlank => pair(lang, "开销项目不能为负数或空白", "Expense items cannot be negative or blank"),
        ErrorKey::AmountNegative => pair(lang, "金额不能为负数", "Amount cannot be negative"),
        ErrorKey::AmountCannotBeNegative => pair(lang, "金额不能为负数", "Amount cannot be negative"),
        ErrorKey::SubsidyCannotBeNegative => pair(lang, "补贴不能为负数", "Subsidy cannot be negative"),
        ErrorKey::SubsidyExceedsTotal => pair(lang, "补贴不能超过开销总额", "Subsidy cannot exceed the total expenses"),
        ErrorKey::SettlementItemsRequired => pair(lang, "请至少填入一项支出", "Please enter at least one expense item"),
        ErrorKey::SelectParticipants => pair(lang, "请选择分摊成员", "Please select participants"),
        ErrorKey::DuplicateParticipant => pair(lang, "参与名单有重复", "Duplicate members in the participant list"),
        ErrorKey::InvalidSplitMode => pair(lang, "分摊方式无效", "Invalid split mode"),
        ErrorKey::MemberAmountRequired => pair(lang, "请为每位成员填写金额", "Please enter an amount for each member"),
        ErrorKey::SplitMismatch { sum, total, diff } => match lang {
            Lang::Zh => format!("分摊金额合计 ({sum}) 与总支出 ({total}) 不符，差额 {diff}"),
            Lang::En => format!("Sum of split amounts ({sum}) does not match total expenditure ({total}), difference {diff}"),
        },
        ErrorKey::CustomSplitMismatch { sum, total } => match lang {
            Lang::Zh => format!("个别金额合计 {sum} 与应分摊总额 {total} 不一致"),
            Lang::En => format!("Individual amounts total {sum} but must equal the shared total {total}"),
        },
        ErrorKey::RevisionSplitMismatch { sum, new_total, diff } => match lang {
            Lang::Zh => format!("修改总支出后分摊金额合计 ({sum}) 与新总支出 ({new_total}) 不符，差额 {diff}"),
            Lang::En => format!("After modifying total expenditure, sum of split amounts ({sum}) does not match new total ({new_total}), difference {diff}"),
        },
        ErrorKey::OnlyDraftsCanBeDeleted => pair(lang, "只有草稿可以删除", "Only drafts can be deleted"),
        ErrorKey::OnlyOrganiserCanEditBill => pair(lang, "只有发起者可以修改", "Only the organiser can edit this"),
        ErrorKey::OnlyOrganiserCanPublish => pair(lang, "只有发起者可以发布", "Only the organiser can publish"),
        ErrorKey::OnlyOrganiserCanCloseBill => pair(lang, "只有发起者可以关闭账单", "Only the organiser can close this bill"),
        ErrorKey::SettlementAlreadyPublished => pair(lang, "此结算已发布", "This settlement has already been published"),
        ErrorKey::RevisionReasonRequired => pair(lang, "发布后修改金额，请填写修改原因", "After publishing, a revision reason is required to change amounts"),
        ErrorKey::UnattendedMembers(count) => match lang {
            Lang::Zh => format!("有 {count} 位成员尚未记录为“已出席”，请先在活动页面核实出席。"),
            Lang::En => format!("{count} member(s) are not recorded as \"attended\" yet; please verify attendance on the event page first."),
        },
        ErrorKey::OpenPaymentsRemain(count) => match lang {
            Lang::Zh => format!("尚有 {count} 位成员未确认付款"),
            Lang::En => format!("{count} member(s) have not confirmed payment yet"),
        },
        ErrorKey::PayeeDetailsRequired => pair(lang, "请填写收款资料（TNG 手机号 / DuitNow / 银行）", "Please fill in payee details (TNG number / DuitNow / bank)"),
        ErrorKey::OnlyPayeeCanVerify => pair(lang, "只有收款者可以核对", "Only the payee can verify"),
        ErrorKey::ProofOnlyForOwnShare => pair(lang, "只能为自己的费用上传凭证", "You can only submit proof for your own share"),
        ErrorKey::ProofNotNeededNow => pair(lang, "此费用目前不需要上传凭证", "Proof upload is not needed for this share at the moment"),
        ErrorKey::UploadReceiptFirst => pair(lang, "请先上传收据", "Please upload the receipt first"),
        ErrorKey::InvalidProofDocument => pair(lang, "凭证无效", "Invalid proof document"),
        ErrorKey::TxnRefRequired => pair(lang, "请填写交易参考编号", "Please enter the transaction reference number"),
        ErrorKey::DuplicateTxnOrProof => pair(lang, "此交易编号或收据已被使用，请确认是否重复上传", "This transaction reference or receipt has already been used; please check for a duplicate upload"),
        ErrorKey::NeedMoreInfoReasonRequired => pair(lang, "请说明需补充的资料", "Please describe what information is needed"),
        ErrorKey::NotOnCostSplitList => pair(lang, "您不在此分摊名单中", "You are not on this cost split's participant list"),
        ErrorKey::BillClosedCannotEdit => pair(lang, "账单已关闭，不能修改", "This bill is closed and can no longer be edited"),
        ErrorKey::OnlyOrganiserCanInitiateCostSplit => pair(lang, "只有活动发起者可以发起费用分摊", "Only the event organiser can create a cost split"),
        other => panic!("{} is not a settlements error", other.code()),
    }
}
