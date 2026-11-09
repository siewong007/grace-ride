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
        other => panic!("{} is not an events error", other.code()),
    }
}
