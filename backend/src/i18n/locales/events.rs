//! Events errors. Both languages live next to each other.

use super::pair;
use crate::i18n::Lang;

pub enum EventsError {
    EventNameRequired,
    InvalidEventType,
    SelectVisibleGroup,
    InvalidEventTime,
    InvalidSignupDeadline,
    DeadlineAfterEvent,
    EventNotOpen,
    SignupClosed,
    DriverRegistrationRequired,
    OnlyOrganiserCanCreate,
    OnlyOrganiserCanEdit,
    OnlyOrganiserCanCancel,
    OnlyOrganiserCanPerformAction,
    OnlyOrganiserCanViewSignups,
    OnlyOrganiserCanRecordAttendance,
    AttendanceModuleDisabled,
    InvalidCheckinCode,
    MembersOnlyAttendance,
    EventNotOpenToGroup,
    JoinAsRequired,
    JoinAsNotAllowed,
    InvalidParentGroup,
    EventRideNeedsSignup,
    EventDriveNeedsSignup,
    DuplicateEventRide,
    DriverOutsideGroup,
    VisitorLinkInvalid,
    VisitorNotOpenYet,
    VisitorLinkUsed,
    VisitorJourneyEnded,
    VisitorNameRequired,
    OnlyGoingCanInviteVisitor,
    InvalidPassCode,
    CarePassClosed,
    MileageAuthorisationRestricted,
}

pub fn render(lang: Lang, key: EventsError) -> String {
    match key {
        EventsError::EventNameRequired => {
            pair(lang, "请填写活动名称", "Please enter an event name")
        }
        EventsError::InvalidEventType => pair(lang, "活动类型无效", "Invalid event type"),
        EventsError::SelectVisibleGroup => pair(
            lang,
            "请至少选择一个可见群体",
            "Please select at least one visible group",
        ),
        EventsError::InvalidEventTime => pair(
            lang,
            "请填写正确的活动时间",
            "Please enter a valid event time",
        ),
        EventsError::InvalidSignupDeadline => pair(
            lang,
            "报名截止时间格式不正确",
            "Signup deadline format is invalid",
        ),
        EventsError::DeadlineAfterEvent => pair(
            lang,
            "报名截止不能晚于活动时间",
            "Signup deadline cannot be later than the event time",
        ),
        EventsError::EventNotOpen => pair(
            lang,
            "此活动不开放报名",
            "This event is not open for signup",
        ),
        EventsError::SignupClosed => pair(lang, "报名已截止", "Signup has closed"),
        EventsError::DriverRegistrationRequired => pair(
            lang,
            "需由交通组登记为志愿司机后才能提供接送",
            "You must be registered by the transport team as a volunteer driver to offer rides",
        ),
        EventsError::OnlyOrganiserCanCreate => pair(
            lang,
            "只有活动负责人可以发起活动",
            "Only event organisers can create events",
        ),
        EventsError::OnlyOrganiserCanEdit => pair(
            lang,
            "只有活动负责人可以修改",
            "Only the event organiser can edit",
        ),
        EventsError::OnlyOrganiserCanCancel => pair(
            lang,
            "只有活动负责人可以取消",
            "Only the event organiser can cancel",
        ),
        EventsError::OnlyOrganiserCanPerformAction => pair(
            lang,
            "只有活动负责人可以操作",
            "Only the event organiser can perform this action",
        ),
        EventsError::OnlyOrganiserCanViewSignups => pair(
            lang,
            "只有活动负责人可以查看报名名单",
            "Only the event organiser can view the signup list",
        ),
        EventsError::OnlyOrganiserCanRecordAttendance => pair(
            lang,
            "只有活动负责人可以记录出席",
            "Only the event organiser can record attendance",
        ),
        EventsError::AttendanceModuleDisabled => pair(
            lang,
            "此活动未开启出席签到模块",
            "The attendance module is not enabled for this event",
        ),
        EventsError::InvalidCheckinCode => pair(
            lang,
            "签到码无效或已更新",
            "The check-in code is invalid or has expired",
        ),
        EventsError::MembersOnlyAttendance => pair(
            lang,
            "出席签到仅限正式会员",
            "Attendance check-in is for members only",
        ),
        EventsError::EventNotOpenToGroup => pair(
            lang,
            "此活动不对您所属的群体开放",
            "This event is not open to your groups",
        ),
        EventsError::JoinAsRequired => pair(
            lang,
            "请选择您以哪个群体参加",
            "Choose which group you are joining as",
        ),
        EventsError::JoinAsNotAllowed => pair(
            lang,
            "您不能以这个群体参加此活动",
            "You can't join this event as that group",
        ),
        EventsError::InvalidParentGroup => pair(
            lang,
            "上级群体无效：不能形成循环，且最多三层",
            "That parent group isn't allowed: no loops, and at most three levels",
        ),
        EventsError::EventDriveNeedsSignup => pair(
            lang,
            "请先报名参加此活动，再自愿接送",
            "Sign up for the event before volunteering to drive for it",
        ),
        EventsError::EventRideNeedsSignup => pair(
            lang,
            "请先报名参加此活动，再申请接送",
            "Sign up for the event before asking for a ride to it",
        ),
        EventsError::DuplicateEventRide => pair(
            lang,
            "您已为此活动申请了接送，请在「我的接送」中查看",
            "You already have a ride request for this event; check My rides",
        ),
        EventsError::DriverOutsideGroup => pair(
            lang,
            "此接送只开放给该群体的司机，交通组可另行指派",
            "Only drivers from this rider's group can take this ride; transport staff can assign others",
        ),
        EventsError::VisitorLinkInvalid => pair(lang, "此链接无效", "This link isn't valid"),
        EventsError::VisitorNotOpenYet => pair(lang, "此链接尚未开放", "This link isn't open yet"),
        EventsError::VisitorLinkUsed => pair(
            lang,
            "此链接已在另一部设备上使用，请向邀请人索取新链接",
            "This link was already used on another device. Ask the person who invited you for a new one",
        ),
        EventsError::VisitorJourneyEnded => pair(
            lang,
            "此行程已结束，感谢您的参与！",
            "This journey has ended. Thank you for coming!",
        ),
        EventsError::VisitorNameRequired => {
            pair(lang, "请填写访客姓名", "Enter the visitor's name")
        }
        EventsError::OnlyGoingCanInviteVisitor => pair(
            lang,
            "只有已报名参加的会员或活动负责人可以邀请访客",
            "Only members going to the event, or its organiser, can invite a visitor",
        ),
        EventsError::InvalidPassCode => pair(
            lang,
            "找不到对应的活动通行码",
            "No matching member pass code found for this event",
        ),
        EventsError::CarePassClosed => pair(
            lang,
            "此平安接送追踪链接已关闭或过期",
            "This safe-arrival status link is closed or expired",
        ),
        EventsError::MileageAuthorisationRestricted => pair(
            lang,
            "只有超级管理员、会长或副会长可以允许活动报销里程车费",
            "Only the super admin, President or Vice-President can allow an event to be claimable for mileage",
        ),
    }
}
