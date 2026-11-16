//! Events errors. Both languages live next to each other.

use super::pair;
use crate::i18n::{keys::ErrorKey, Lang};

pub fn render(lang: Lang, key: &ErrorKey) -> String {
    match key {
        ErrorKey::EventNameRequired => pair(lang, "请填写活动名称", "Please enter an event name"),
        ErrorKey::InvalidEventType => pair(lang, "活动类型无效", "Invalid event type"),
        ErrorKey::SelectVisibleGroup => pair(
            lang,
            "请至少选择一个可见群体",
            "Please select at least one visible group",
        ),
        ErrorKey::InvalidEventTime => pair(
            lang,
            "请填写正确的活动时间",
            "Please enter a valid event time",
        ),
        ErrorKey::InvalidSignupDeadline => pair(
            lang,
            "报名截止时间格式不正确",
            "Signup deadline format is invalid",
        ),
        ErrorKey::DeadlineAfterEvent => pair(
            lang,
            "报名截止不能晚于活动时间",
            "Signup deadline cannot be later than the event time",
        ),
        ErrorKey::EventNotOpen => pair(
            lang,
            "此活动不开放报名",
            "This event is not open for signup",
        ),
        ErrorKey::SignupClosed => pair(lang, "报名已截止", "Signup has closed"),
        ErrorKey::DriverRegistrationRequired => pair(
            lang,
            "需由交通组登记为志愿司机后才能提供接送",
            "You must be registered by the transport team as a volunteer driver to offer rides",
        ),
        ErrorKey::OnlyOrganiserCanCreate => pair(
            lang,
            "只有活动负责人可以发起活动",
            "Only event organisers can create events",
        ),
        ErrorKey::OnlyOrganiserCanEdit => pair(
            lang,
            "只有活动负责人可以修改",
            "Only the event organiser can edit",
        ),
        ErrorKey::OnlyOrganiserCanCancel => pair(
            lang,
            "只有活动负责人可以取消",
            "Only the event organiser can cancel",
        ),
        ErrorKey::OnlyOrganiserCanPerformAction => pair(
            lang,
            "只有活动负责人可以操作",
            "Only the event organiser can perform this action",
        ),
        ErrorKey::OnlyOrganiserCanViewSignups => pair(
            lang,
            "只有活动负责人可以查看报名名单",
            "Only the event organiser can view the signup list",
        ),
        ErrorKey::OnlyOrganiserCanRecordAttendance => pair(
            lang,
            "只有活动负责人可以记录出席",
            "Only the event organiser can record attendance",
        ),
        ErrorKey::EventNotOpenToGroup => pair(
            lang,
            "此活动不对您所属的群体开放",
            "This event is not open to your groups",
        ),
        ErrorKey::JoinAsRequired => pair(
            lang,
            "请选择您以哪个群体参加",
            "Choose which group you are joining as",
        ),
        ErrorKey::JoinAsNotAllowed => pair(
            lang,
            "您不能以这个群体参加此活动",
            "You can't join this event as that group",
        ),
        ErrorKey::InvalidParentGroup => pair(
            lang,
            "上级群体无效：不能形成循环，且最多三层",
            "That parent group isn't allowed: no loops, and at most three levels",
        ),
        ErrorKey::EventRideNeedsSignup => pair(
            lang,
            "请先报名参加此活动，再申请接送",
            "Sign up for the event before asking for a ride to it",
        ),
        ErrorKey::DriverOutsideGroup => pair(
            lang,
            "此接送只开放给该群体的司机，交通组可另行指派",
            "Only drivers from this rider's group can take this ride; transport staff can assign others",
        ),
        ErrorKey::VisitorLinkInvalid => pair(
            lang,
            "此链接无效",
            "This link isn't valid",
        ),
        ErrorKey::VisitorNotOpenYet => pair(
            lang,
            "此链接尚未开放",
            "This link isn't open yet",
        ),
        ErrorKey::VisitorLinkUsed => pair(
            lang,
            "此链接已在另一部设备上使用，请向邀请人索取新链接",
            "This link was already used on another device. Ask the person who invited you for a new one",
        ),
        ErrorKey::VisitorJourneyEnded => pair(
            lang,
            "此行程已结束，感谢您的参与！",
            "This journey has ended. Thank you for coming!",
        ),
        ErrorKey::VisitorNameRequired => pair(
            lang,
            "请填写访客姓名",
            "Enter the visitor's name",
        ),
        ErrorKey::OnlyGoingCanInviteVisitor => pair(
            lang,
            "只有已报名参加的会员或活动负责人可以邀请访客",
            "Only members going to the event, or its organiser, can invite a visitor",
        ),
        other => panic!("{} is not an events error", other.code()),
    }
}
