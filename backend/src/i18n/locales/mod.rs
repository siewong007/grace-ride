//! Error messages grouped by error type.
//!
//! Each module owns one type and keeps 中文 and English together.
//! `render` is exhaustive over `ErrorKey`, so a new code cannot ship without a message.

pub mod admin;
pub mod auth;
pub mod common;
pub mod community;
pub mod events;
pub mod files;
pub mod rides;
pub mod settlements;

use crate::i18n::{keys::ErrorKey, Lang};

pub fn pair(lang: Lang, zh: &'static str, en: &'static str) -> String {
    crate::i18n::t(lang, zh, en).to_string()
}

pub fn render(lang: Lang, key: &ErrorKey) -> String {
    match key {
        ErrorKey::NotFound
        | ErrorKey::Unauthorized
        | ErrorKey::Forbidden
        | ErrorKey::ServerError
        | ErrorKey::InvalidAction
        | ErrorKey::InvalidStatus
        | ErrorKey::InvalidDate
        | ErrorKey::NoPermission
        | ErrorKey::MfaRequired
        | ErrorKey::RateLimited => common::render(lang, key),
        ErrorKey::CannotDisableSelf
        | ErrorKey::CannotGrantSelfFinance
        | ErrorKey::SecondAdminRequired
        | ErrorKey::CannotDeleteAllMembersGroup
        | ErrorKey::GroupNameRequired
        | ErrorKey::GroupNameExists
        | ErrorKey::AreaNameRequired
        | ErrorKey::AreaExists
        | ErrorKey::RequestAlreadyHandled
        | ErrorKey::EmailInvalid
        | ErrorKey::EmailAlreadyMember
        | ErrorKey::UnknownRole
        | ErrorKey::RoleNameRequired
        | ErrorKey::RoleNameExists
        | ErrorKey::InvalidPermission
        | ErrorKey::RolePermissionLocked
        | ErrorKey::SystemRoleCannotBeDeleted
        | ErrorKey::CannotRemoveOwnRoleAccess
        | ErrorKey::SuperadminOnly
        | ErrorKey::LastSuperadmin => admin::render(lang, key),
        ErrorKey::InvalidCredentials
        | ErrorKey::VerificationExpired
        | ErrorKey::InvalidCode
        | ErrorKey::CodeAlreadyUsed
        | ErrorKey::Invalid2FaSecret
        | ErrorKey::TwoFactorAlreadyEnabled
        | ErrorKey::TwoFactorNotConfigured
        | ErrorKey::StartSetupFirst
        | ErrorKey::PasswordTooShort
        | ErrorKey::CurrentPasswordIncorrect
        | ErrorKey::InviteExpired
        | ErrorKey::InviteAlreadyUsed
        | ErrorKey::RecoveryLinkInvalid
        | ErrorKey::PasskeyNotFound
        | ErrorKey::PasskeyFailed
        | ErrorKey::PasskeyRegistrationFailed(_)
        | ErrorKey::ChallengeMismatch
        | ErrorKey::NameRequired
        | ErrorKey::NameTooLong
        | ErrorKey::PhoneInvalid
        | ErrorKey::RecentSignInRequired
        | ErrorKey::EmailAlreadyRegistered
        | ErrorKey::TooManyFailedLogins
        | ErrorKey::AuthDisabled
        | ErrorKey::AccountPending
        | ErrorKey::TermsRequired => auth::render(lang, key),
        ErrorKey::EventNameRequired
        | ErrorKey::InvalidEventType
        | ErrorKey::SelectVisibleGroup
        | ErrorKey::InvalidEventTime
        | ErrorKey::InvalidSignupDeadline
        | ErrorKey::DeadlineAfterEvent
        | ErrorKey::EventNotOpen
        | ErrorKey::SignupClosed
        | ErrorKey::DriverRegistrationRequired
        | ErrorKey::OnlyOrganiserCanCreate
        | ErrorKey::OnlyOrganiserCanEdit
        | ErrorKey::OnlyOrganiserCanCancel
        | ErrorKey::OnlyOrganiserCanPerformAction
        | ErrorKey::OnlyOrganiserCanViewSignups
        | ErrorKey::OnlyOrganiserCanRecordAttendance
        | ErrorKey::EventNotOpenToGroup => events::render(lang, key),
        ErrorKey::PickupTimeRequired
        | ErrorKey::InvalidPickupTime
        | ErrorKey::PickupTimeInPast
        | ErrorKey::SeatsRange
        | ErrorKey::AddressAndDestinationRequired
        | ErrorKey::PickupAreaRequired
        | ErrorKey::RiderNameRequired
        | ErrorKey::CannotViewEvent
        | ErrorKey::DriverCannotClaimOwnRide
        | ErrorKey::CannotPickOwnRide
        | ErrorKey::FillVehicleDetailsFirst
        | ErrorKey::NotEnoughSeatsSlot { .. }
        | ErrorKey::RidePickedByAnotherDriver
        | ErrorKey::DriverAlreadyClaimed
        | ErrorKey::DriverAlreadyAssigned
        | ErrorKey::CannotClaimMultipleDrivers
        | ErrorKey::RideNotAvailableForClaim
        | ErrorKey::OnlyPassengerCanRespond
        | ErrorKey::TripAlreadyApproved
        | ErrorKey::TripNotApproved
        | ErrorKey::TripAlreadyCancelled
        | ErrorKey::TripAlreadyCompleted
        | ErrorKey::DriverCannotDrive
        | ErrorKey::StartTripFirst
        | ErrorKey::InvalidLocation
        | ErrorKey::MaxLiveDriversReached { .. }
        | ErrorKey::RideStatusChanged
        | ErrorKey::GroupChatClosed
        | ErrorKey::GroupChatNotApproved
        | ErrorKey::OnlyTripMembersCanViewChat
        | ErrorKey::InvalidMessageLength
        | ErrorKey::NoDriverYet
        | ErrorKey::DriverNotFound
        | ErrorKey::DriverScheduleMismatch
        | ErrorKey::DriverCapacityMismatch
        | ErrorKey::DriverStatusInvalid
        | ErrorKey::DriverAlreadyOnTrip
        | ErrorKey::DriverAssignedOtherRide
        | ErrorKey::NotEnoughDriverSeats { .. }
        | ErrorKey::NotEnoughDriverSeatsReassign { .. }
        | ErrorKey::NotRideDriver
        | ErrorKey::NotRidePassenger
        | ErrorKey::CannotViewLocation
        | ErrorKey::CannotViewRide
        | ErrorKey::TripResponseNotAllowed
        | ErrorKey::OnlyVolunteerDriversCanSet
        | ErrorKey::SpareSeatsRange
        | ErrorKey::VehicleModelAndPlateRequired
        | ErrorKey::VehicleSeatsRange
        | ErrorKey::MemberNotVolunteerDriver
        | ErrorKey::DriverCannotBePassenger
        | ErrorKey::OnlyPassengerOrStaffCanCancel
        | ErrorKey::OnlyVolunteerDriversCanView
        | ErrorKey::OnlyVolunteerDriversCanChoose => rides::render(lang, key),
        ErrorKey::SettlementNameRequired
        | ErrorKey::AtLeastOneExpenseItem
        | ErrorKey::ExpenseItemNegativeOrBlank
        | ErrorKey::AmountNegative
        | ErrorKey::AmountCannotBeNegative
        | ErrorKey::SubsidyCannotBeNegative
        | ErrorKey::SubsidyExceedsTotal
        | ErrorKey::SettlementItemsRequired
        | ErrorKey::SelectParticipants
        | ErrorKey::DuplicateParticipant
        | ErrorKey::InvalidSplitMode
        | ErrorKey::MemberAmountRequired
        | ErrorKey::SplitMismatch { .. }
        | ErrorKey::CustomSplitMismatch { .. }
        | ErrorKey::RevisionSplitMismatch { .. }
        | ErrorKey::OnlyDraftsCanBeDeleted
        | ErrorKey::OnlyOrganiserCanEditBill
        | ErrorKey::OnlyOrganiserCanPublish
        | ErrorKey::OnlyOrganiserCanCloseBill
        | ErrorKey::SettlementAlreadyPublished
        | ErrorKey::RevisionReasonRequired
        | ErrorKey::UnattendedMembers(_)
        | ErrorKey::OpenPaymentsRemain(_)
        | ErrorKey::PayeeDetailsRequired
        | ErrorKey::OnlyPayeeCanVerify
        | ErrorKey::ProofOnlyForOwnShare
        | ErrorKey::ProofNotNeededNow
        | ErrorKey::UploadReceiptFirst
        | ErrorKey::InvalidProofDocument
        | ErrorKey::TxnRefRequired
        | ErrorKey::DuplicateTxnOrProof
        | ErrorKey::NeedMoreInfoReasonRequired
        | ErrorKey::NotOnCostSplitList
        | ErrorKey::BillClosedCannotEdit
        | ErrorKey::OnlyOrganiserCanInitiateCostSplit => settlements::render(lang, key),
        ErrorKey::QuestionAndDetailsRequired
        | ErrorKey::RequestAlreadyClosed
        | ErrorKey::ReplyContentRequired
        | ErrorKey::LanguageInvalid
        | ErrorKey::TitleAndContentRequired
        | ErrorKey::GuideTitleRequired
        | ErrorKey::GuideAudienceInvalid
        | ErrorKey::GuideSlugRequired
        | ErrorKey::GuideSlugExists
        | ErrorKey::GuideLinkInvalid
        | ErrorKey::GuideStepInvalid
        | ErrorKey::OnlyLinkOwnRides
        | ErrorKey::AmountRangeRM001to500
        | ErrorKey::PayeeNameAndPhoneRequired
        | ErrorKey::ReceiptRequired
        | ErrorKey::ReimbursementOnlyPassenger
        | ErrorKey::ReimbursementOnlyNonPending
        | ErrorKey::OnlyReturnedReimbCanBeResubmitted
        | ErrorKey::ReimbAlreadyReviewed
        | ErrorKey::CannotReviewOwnRequest
        | ErrorKey::OnlyApprovedReimbCanBePaid
        | ErrorKey::CannotPayOwnRequest
        | ErrorKey::PayeeVerificationRequired
        | ErrorKey::ReimbursementProcessed
        | ErrorKey::ReasonRequired
        | ErrorKey::MonthFormatInvalid => community::render(lang, key),
        ErrorKey::NoFileReceived
        | ErrorKey::FileTooLarge
        | ErrorKey::InvalidFileType
        | ErrorKey::UploadFailed
        | ErrorKey::UploadQuotaReached
        | ErrorKey::InvalidReceipt
        | ErrorKey::InvalidPhoto
        | ErrorKey::CannotViewFile => files::render(lang, key),
    }
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::i18n::keys::ErrorKey;
    use crate::i18n::Lang;

    fn samples() -> Vec<ErrorKey> {
        vec![
            ErrorKey::NotFound,
            ErrorKey::Unauthorized,
            ErrorKey::Forbidden,
            ErrorKey::ServerError,
            ErrorKey::InvalidAction,
            ErrorKey::InvalidStatus,
            ErrorKey::InvalidDate,
            ErrorKey::NoPermission,
            ErrorKey::MfaRequired,
            ErrorKey::RateLimited,
            ErrorKey::CannotDisableSelf,
            ErrorKey::CannotDeleteAllMembersGroup,
            ErrorKey::GroupNameRequired,
            ErrorKey::GroupNameExists,
            ErrorKey::AreaNameRequired,
            ErrorKey::AreaExists,
            ErrorKey::RequestAlreadyHandled,
            ErrorKey::EmailInvalid,
            ErrorKey::EmailAlreadyMember,
            ErrorKey::UnknownRole,
            ErrorKey::RoleNameRequired,
            ErrorKey::RoleNameExists,
            ErrorKey::InvalidPermission,
            ErrorKey::RolePermissionLocked,
            ErrorKey::SystemRoleCannotBeDeleted,
            ErrorKey::CannotRemoveOwnRoleAccess,
            ErrorKey::SuperadminOnly,
            ErrorKey::LastSuperadmin,
            ErrorKey::InvalidCredentials,
            ErrorKey::VerificationExpired,
            ErrorKey::InvalidCode,
            ErrorKey::CodeAlreadyUsed,
            ErrorKey::Invalid2FaSecret,
            ErrorKey::TwoFactorAlreadyEnabled,
            ErrorKey::TwoFactorNotConfigured,
            ErrorKey::StartSetupFirst,
            ErrorKey::PasswordTooShort,
            ErrorKey::CurrentPasswordIncorrect,
            ErrorKey::InviteExpired,
            ErrorKey::InviteAlreadyUsed,
            ErrorKey::RecoveryLinkInvalid,
            ErrorKey::PasskeyNotFound,
            ErrorKey::PasskeyFailed,
            ErrorKey::PasskeyRegistrationFailed("x".into()),
            ErrorKey::ChallengeMismatch,
            ErrorKey::NameRequired,
            ErrorKey::NameTooLong,
            ErrorKey::PhoneInvalid,
            ErrorKey::RecentSignInRequired,
            ErrorKey::EmailAlreadyRegistered,
            ErrorKey::TooManyFailedLogins,
            ErrorKey::AuthDisabled,
            ErrorKey::AccountPending,
            ErrorKey::TermsRequired,
            ErrorKey::EventNameRequired,
            ErrorKey::InvalidEventType,
            ErrorKey::SelectVisibleGroup,
            ErrorKey::InvalidEventTime,
            ErrorKey::InvalidSignupDeadline,
            ErrorKey::DeadlineAfterEvent,
            ErrorKey::EventNotOpen,
            ErrorKey::SignupClosed,
            ErrorKey::DriverRegistrationRequired,
            ErrorKey::OnlyOrganiserCanCreate,
            ErrorKey::OnlyOrganiserCanEdit,
            ErrorKey::OnlyOrganiserCanCancel,
            ErrorKey::OnlyOrganiserCanPerformAction,
            ErrorKey::OnlyOrganiserCanViewSignups,
            ErrorKey::OnlyOrganiserCanRecordAttendance,
            ErrorKey::EventNotOpenToGroup,
            ErrorKey::PickupTimeRequired,
            ErrorKey::InvalidPickupTime,
            ErrorKey::PickupTimeInPast,
            ErrorKey::SeatsRange,
            ErrorKey::AddressAndDestinationRequired,
            ErrorKey::PickupAreaRequired,
            ErrorKey::RiderNameRequired,
            ErrorKey::CannotViewEvent,
            ErrorKey::DriverCannotClaimOwnRide,
            ErrorKey::CannotPickOwnRide,
            ErrorKey::FillVehicleDetailsFirst,
            ErrorKey::NotEnoughSeatsSlot { load: 1, cap: 1 },
            ErrorKey::RidePickedByAnotherDriver,
            ErrorKey::DriverAlreadyClaimed,
            ErrorKey::DriverAlreadyAssigned,
            ErrorKey::CannotClaimMultipleDrivers,
            ErrorKey::RideNotAvailableForClaim,
            ErrorKey::OnlyPassengerCanRespond,
            ErrorKey::TripAlreadyApproved,
            ErrorKey::TripNotApproved,
            ErrorKey::TripAlreadyCancelled,
            ErrorKey::TripAlreadyCompleted,
            ErrorKey::DriverCannotDrive,
            ErrorKey::StartTripFirst,
            ErrorKey::InvalidLocation,
            ErrorKey::MaxLiveDriversReached { live: 1, limit: 1 },
            ErrorKey::RideStatusChanged,
            ErrorKey::GroupChatClosed,
            ErrorKey::GroupChatNotApproved,
            ErrorKey::OnlyTripMembersCanViewChat,
            ErrorKey::InvalidMessageLength,
            ErrorKey::NoDriverYet,
            ErrorKey::DriverNotFound,
            ErrorKey::DriverScheduleMismatch,
            ErrorKey::DriverCapacityMismatch,
            ErrorKey::DriverStatusInvalid,
            ErrorKey::DriverAlreadyOnTrip,
            ErrorKey::DriverAssignedOtherRide,
            ErrorKey::NotEnoughDriverSeats { load: 1, cap: 1 },
            ErrorKey::NotEnoughDriverSeatsReassign { load: 1, cap: 1 },
            ErrorKey::NotRideDriver,
            ErrorKey::NotRidePassenger,
            ErrorKey::CannotViewLocation,
            ErrorKey::CannotViewRide,
            ErrorKey::TripResponseNotAllowed,
            ErrorKey::OnlyVolunteerDriversCanSet,
            ErrorKey::SpareSeatsRange,
            ErrorKey::VehicleModelAndPlateRequired,
            ErrorKey::VehicleSeatsRange,
            ErrorKey::MemberNotVolunteerDriver,
            ErrorKey::DriverCannotBePassenger,
            ErrorKey::OnlyPassengerOrStaffCanCancel,
            ErrorKey::OnlyVolunteerDriversCanView,
            ErrorKey::OnlyVolunteerDriversCanChoose,
            ErrorKey::SettlementNameRequired,
            ErrorKey::AtLeastOneExpenseItem,
            ErrorKey::ExpenseItemNegativeOrBlank,
            ErrorKey::AmountNegative,
            ErrorKey::AmountCannotBeNegative,
            ErrorKey::SubsidyCannotBeNegative,
            ErrorKey::SubsidyExceedsTotal,
            ErrorKey::SettlementItemsRequired,
            ErrorKey::SelectParticipants,
            ErrorKey::DuplicateParticipant,
            ErrorKey::InvalidSplitMode,
            ErrorKey::MemberAmountRequired,
            ErrorKey::SplitMismatch {
                sum: "1".into(),
                total: "1".into(),
                diff: "1".into(),
            },
            ErrorKey::CustomSplitMismatch {
                sum: "1".into(),
                total: "1".into(),
            },
            ErrorKey::RevisionSplitMismatch {
                sum: "1".into(),
                new_total: "1".into(),
                diff: "1".into(),
            },
            ErrorKey::OnlyDraftsCanBeDeleted,
            ErrorKey::OnlyOrganiserCanEditBill,
            ErrorKey::OnlyOrganiserCanPublish,
            ErrorKey::OnlyOrganiserCanCloseBill,
            ErrorKey::SettlementAlreadyPublished,
            ErrorKey::RevisionReasonRequired,
            ErrorKey::UnattendedMembers(1),
            ErrorKey::OpenPaymentsRemain(1),
            ErrorKey::PayeeDetailsRequired,
            ErrorKey::OnlyPayeeCanVerify,
            ErrorKey::ProofOnlyForOwnShare,
            ErrorKey::ProofNotNeededNow,
            ErrorKey::UploadReceiptFirst,
            ErrorKey::InvalidProofDocument,
            ErrorKey::TxnRefRequired,
            ErrorKey::DuplicateTxnOrProof,
            ErrorKey::NeedMoreInfoReasonRequired,
            ErrorKey::NotOnCostSplitList,
            ErrorKey::BillClosedCannotEdit,
            ErrorKey::OnlyOrganiserCanInitiateCostSplit,
            ErrorKey::QuestionAndDetailsRequired,
            ErrorKey::RequestAlreadyClosed,
            ErrorKey::ReplyContentRequired,
            ErrorKey::LanguageInvalid,
            ErrorKey::TitleAndContentRequired,
            ErrorKey::GuideTitleRequired,
            ErrorKey::GuideAudienceInvalid,
            ErrorKey::GuideSlugRequired,
            ErrorKey::GuideSlugExists,
            ErrorKey::GuideLinkInvalid,
            ErrorKey::GuideStepInvalid,
            ErrorKey::OnlyLinkOwnRides,
            ErrorKey::AmountRangeRM001to500,
            ErrorKey::PayeeNameAndPhoneRequired,
            ErrorKey::ReceiptRequired,
            ErrorKey::ReimbursementOnlyPassenger,
            ErrorKey::ReimbursementOnlyNonPending,
            ErrorKey::OnlyReturnedReimbCanBeResubmitted,
            ErrorKey::ReimbAlreadyReviewed,
            ErrorKey::CannotReviewOwnRequest,
            ErrorKey::OnlyApprovedReimbCanBePaid,
            ErrorKey::CannotPayOwnRequest,
            ErrorKey::PayeeVerificationRequired,
            ErrorKey::ReimbursementProcessed,
            ErrorKey::ReasonRequired,
            ErrorKey::MonthFormatInvalid,
            ErrorKey::NoFileReceived,
            ErrorKey::FileTooLarge,
            ErrorKey::InvalidFileType,
            ErrorKey::UploadFailed,
            ErrorKey::UploadQuotaReached,
            ErrorKey::InvalidReceipt,
            ErrorKey::InvalidPhoto,
            ErrorKey::CannotViewFile,
        ]
    }

    #[test]
    fn every_error_has_a_unique_code_and_both_languages() {
        let dynamic = [
            "PasskeyRegistrationFailed",
            "NotEnoughSeatsSlot",
            "MaxLiveDriversReached",
            "NotEnoughDriverSeats",
            "NotEnoughDriverSeatsReassign",
            "SplitMismatch",
            "CustomSplitMismatch",
            "RevisionSplitMismatch",
            "UnattendedMembers",
            "OpenPaymentsRemain",
        ];
        let mut seen = std::collections::HashSet::new();
        for key in samples() {
            assert!(
                seen.insert(key.code()),
                "duplicate error code {}",
                key.code()
            );
            let zh = render(Lang::Zh, &key);
            let en = render(Lang::En, &key);
            assert!(!zh.is_empty() && !en.is_empty(), "{}", key.code());
            assert_ne!(zh, en, "{}", key.code());
            let named = format!("{key:?}");
            let is_dynamic = dynamic.iter().any(|n| {
                named.strip_prefix(*n).is_some_and(|rest| {
                    rest.starts_with(' ') || rest.starts_with('(') || rest.starts_with('{')
                })
            });
            assert_eq!(key.params().is_some(), is_dynamic, "{}", key.code());
        }
    }
}
