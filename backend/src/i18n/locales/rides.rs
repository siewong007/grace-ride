//! Rides errors. Both languages live next to each other.

use super::pair;
use crate::i18n::Lang;

pub enum RidesError {
    InvalidPickupTime,
    PickupTimeInPast,
    SeatsRange,
    AddressAndDestinationRequired,
    PickupAreaRequired,
    RiderNameRequired,
    CannotViewEvent,
    CannotPickOwnRide,
    FillVehicleDetailsFirst,
    NotEnoughSeatsSlot { load: i64, cap: i64 },
    RidePickedByAnotherDriver,
    OnlyPassengerCanRespond,
    TripAlreadyCompleted,
    StartTripFirst,
    PassengerAlreadyPickedUp,
    InvalidLocation,
    MaxLiveDriversReached { live: i64, limit: i64 },
    RideStatusChanged,
    GroupChatClosed,
    GroupChatNotApproved,
    OnlyTripMembersCanViewChat,
    InvalidMessageLength,
    InvalidReaction,
    NoDriverYet,
    DriverNotFound,
    NotEnoughDriverSeats { load: i64, cap: i64 },
    NotEnoughDriverSeatsReassign { load: i64, cap: i64 },
    NotRideDriver,
    NotRidePassenger,
    CannotViewLocation,
    CannotViewRide,
    OnlyVolunteerDriversCanSet,
    SpareSeatsRange,
    VehicleModelAndPlateRequired,
    VehicleSeatsRange,
    MemberNotVolunteerDriver,
    DriverCannotBePassenger,
    OnlyPassengerOrStaffCanCancel,
    OnlyVolunteerDriversCanView,
    OnlyVolunteerDriversCanChoose,
    OnlyTrafficLeadCanApprove,
    PlanDraftNotFound,
}

pub fn render(lang: Lang, key: RidesError) -> String {
    match key {
        RidesError::InvalidPickupTime => pair(
            lang,
            "请填写正确的接送时间",
            "Please enter a valid pickup time",
        ),
        RidesError::PickupTimeInPast => {
            pair(lang, "接送时间不能早于现在", "Pickup time cannot be in the past")
        }
        RidesError::SeatsRange => pair(lang, "乘车人数需为 1–6 位", "Seats must be between 1 and 6"),
        RidesError::AddressAndDestinationRequired => pair(
            lang,
            "请填写上车地址及目的地",
            "Please enter pickup address and destination",
        ),
        RidesError::PickupAreaRequired => {
            pair(lang, "请选择接送地区", "Please choose a pickup area")
        }
        RidesError::RiderNameRequired => pair(
            lang,
            "代订时必须填写实际乘车人姓名",
            "Rider name is required when booking for others",
        ),
        RidesError::CannotViewEvent => pair(lang, "您无法查看此活动", "You cannot view this event"),
        RidesError::CannotPickOwnRide => pair(
            lang,
            "不能选择自己的接送报名",
            "You cannot pick your own ride request",
        ),
        RidesError::FillVehicleDetailsFirst => pair(
            lang,
            "请先在司机设置中填写车辆资料",
            "Please add your vehicle details in driver settings first",
        ),
        RidesError::NotEnoughSeatsSlot { load, cap } => match lang {
            Lang::Zh => format!("座位不足：该时段已安排 {load} 位，可提供 {cap} 位。如需多载，请在司机设置开启共乘模式。"),
            Lang::En => format!("Not enough seats: {load} booked in this slot, {cap} available. Enable carpool mode in driver settings to carry more."),
        },
        RidesError::RidePickedByAnotherDriver => pair(
            lang,
            "此报名已被其他司机选择",
            "This request has been picked by another driver",
        ),
        RidesError::OnlyPassengerCanRespond => {
            pair(lang, "只有乘客本人可以回应", "Only the passenger can respond")
        }
        RidesError::TripAlreadyCompleted => {
            pair(lang, "行程已完成", "This ride has been completed")
        }
        RidesError::StartTripFirst => pair(lang, "请先开始行程", "Start the trip first"),
        RidesError::PassengerAlreadyPickedUp => pair(
            lang,
            "乘客已上车，不能记录为未到场",
            "The passenger has been picked up and can't be marked as a no-show",
        ),
        RidesError::InvalidLocation => pair(lang, "位置无效", "Invalid location"),
        RidesError::MaxLiveDriversReached { live, limit } => match lang {
            Lang::Zh => format!("目前已有 {live} 位司机在行程中（上限 {limit}），请稍后或联系交通组。"),
            Lang::En => format!("There are already {live} drivers on trips (limit {limit}); try again later or contact transport staff."),
        },
        RidesError::RideStatusChanged => pair(
            lang,
            "接送状态已改变，请重新整理",
            "Ride status has changed, please refresh",
        ),
        RidesError::GroupChatClosed => pair(
            lang,
            "本趟群聊已关闭",
            "The group chat for this trip is closed",
        ),
        RidesError::GroupChatNotApproved => pair(
            lang,
            "行程获批准后才开放群聊",
            "Group chat opens once the trip is approved",
        ),
        RidesError::OnlyTripMembersCanViewChat => pair(
            lang,
            "只有本趟成员可以查看群聊",
            "Only members of this trip can view the chat",
        ),
        RidesError::InvalidMessageLength => {
            pair(lang, "讯息长度不正确", "Message length is invalid")
        }
        RidesError::InvalidReaction => pair(lang, "无法使用此表情回应", "That reaction isn't available"),
        RidesError::NoDriverYet => pair(lang, "尚未有司机", "No driver yet"),
        RidesError::DriverNotFound => pair(lang, "找不到此司机", "Driver not found"),
        RidesError::NotEnoughDriverSeats { load, cap } => match lang {
            Lang::Zh => format!("司机座位不足：已安排 {load} 位，可提供 {cap} 位"),
            Lang::En => format!("Not enough driver seats: {load} booked, {cap} available"),
        },
        RidesError::NotEnoughDriverSeatsReassign { load, cap } => match lang {
            Lang::Zh => format!("该司机座位不足：已安排 {load} 位，可提供 {cap} 位"),
            Lang::En => {
                format!("Not enough seats for this driver: {load} booked, {cap} available")
            }
        },
        RidesError::NotRideDriver => pair(
            lang,
            "您不是此接送的司机",
            "You are not the driver of this ride",
        ),
        RidesError::NotRidePassenger => pair(
            lang,
            "您不是此接送的乘客",
            "You are not the passenger of this ride",
        ),
        RidesError::CannotViewLocation => {
            pair(lang, "您无法查看此位置", "You cannot view this location")
        }
        RidesError::CannotViewRide => pair(lang, "您无法查看此接送", "You cannot view this ride"),
        RidesError::OnlyVolunteerDriversCanSet => pair(
            lang,
            "只有志愿司机可以设置",
            "Only volunteer drivers can change settings",
        ),
        RidesError::SpareSeatsRange => {
            pair(lang, "空余座位需为 1–7", "Spare seats must be between 1 and 7")
        }
        RidesError::VehicleModelAndPlateRequired => {
            pair(lang, "请填写车型及车牌", "Please enter vehicle model and plate")
        }
        RidesError::VehicleSeatsRange => pair(
            lang,
            "车辆座位数需为 2–9",
            "Vehicle seats must be between 2 and 9",
        ),
        RidesError::MemberNotVolunteerDriver => pair(
            lang,
            "此会员不是志愿司机",
            "This member is not a volunteer driver",
        ),
        RidesError::DriverCannotBePassenger => pair(
            lang,
            "司机不能是乘客本人",
            "The driver cannot be the passenger themselves",
        ),
        RidesError::OnlyPassengerOrStaffCanCancel => pair(
            lang,
            "只有乘客或交通组可以取消",
            "Only the passenger or transport staff can cancel",
        ),
        RidesError::OnlyVolunteerDriversCanView => {
            pair(lang, "只有志愿司机可以查看", "Only volunteer drivers can view")
        }
        RidesError::OnlyVolunteerDriversCanChoose => pair(
            lang,
            "只有志愿司机可以选择接送",
            "Only volunteer drivers can choose rides",
        ),
        RidesError::OnlyTrafficLeadCanApprove => pair(
            lang,
            "只有交通组组长可以审批接送安排",
            "Only the traffic lead can approve transport arrangements",
        ),
        RidesError::PlanDraftNotFound => pair(
            lang,
            "未找到接送安排草稿",
            "Transport arrangement draft not found",
        ),
    }
}
