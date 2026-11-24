//! Error messages grouped by domain.
//!
//! Each module owns a domain sub-enum and keeps 中文 and English together.
//! Both `render` and every domain `render` are 100% exhaustive over their enums —
//! no wildcard or runtime `panic!` catch-all exists anywhere.

pub mod admin;
pub mod auth;
pub mod common;
pub mod community;
pub mod events;
pub mod family;
pub mod files;
pub mod rides;
pub mod settlements;

use crate::i18n::{keys::ErrorKey, Lang};
use admin::AdminError;
use auth::AuthError;
use common::CommonError;
use community::CommunityError;
use events::EventsError;
use family::FamilyError;
use files::FilesError;
use rides::RidesError;
use settlements::SettlementsError;

pub fn pair(lang: Lang, zh: &'static str, en: &'static str) -> String {
    crate::i18n::t(lang, zh, en).to_string()
}

pub fn render(lang: Lang, key: &ErrorKey) -> String {
    match key {
        ErrorKey::NotFound => common::render(lang, CommonError::NotFound),
        ErrorKey::Unauthorized => common::render(lang, CommonError::Unauthorized),
        ErrorKey::Forbidden => common::render(lang, CommonError::Forbidden),
        ErrorKey::ServerError => common::render(lang, CommonError::ServerError),
        ErrorKey::InvalidAction => common::render(lang, CommonError::InvalidAction),
        ErrorKey::InvalidStatus => common::render(lang, CommonError::InvalidStatus),
        ErrorKey::InvalidDate => common::render(lang, CommonError::InvalidDate),
        ErrorKey::NoPermission => common::render(lang, CommonError::NoPermission),
        ErrorKey::MfaRequired => common::render(lang, CommonError::MfaRequired),
        ErrorKey::RateLimited => common::render(lang, CommonError::RateLimited),

        ErrorKey::CannotDisableSelf => admin::render(lang, AdminError::CannotDisableSelf),
        ErrorKey::CannotGrantSelfFinance => admin::render(lang, AdminError::CannotGrantSelfFinance),
        ErrorKey::SecondAdminRequired => admin::render(lang, AdminError::SecondAdminRequired),
        ErrorKey::CannotDeleteAllMembersGroup => {
            admin::render(lang, AdminError::CannotDeleteAllMembersGroup)
        }
        ErrorKey::GroupNameRequired => admin::render(lang, AdminError::GroupNameRequired),
        ErrorKey::GroupNameExists => admin::render(lang, AdminError::GroupNameExists),
        ErrorKey::AreaNameRequired => admin::render(lang, AdminError::AreaNameRequired),
        ErrorKey::AreaExists => admin::render(lang, AdminError::AreaExists),
        ErrorKey::AreaPhotoLimit => admin::render(lang, AdminError::AreaPhotoLimit),
        ErrorKey::AreaPhotoCaptionTooLong => {
            admin::render(lang, AdminError::AreaPhotoCaptionTooLong)
        }
        ErrorKey::RequestAlreadyHandled => admin::render(lang, AdminError::RequestAlreadyHandled),
        ErrorKey::EmailInvalid => admin::render(lang, AdminError::EmailInvalid),
        ErrorKey::UnknownRole => admin::render(lang, AdminError::UnknownRole),
        ErrorKey::RoleNameRequired => admin::render(lang, AdminError::RoleNameRequired),
        ErrorKey::RoleNameExists => admin::render(lang, AdminError::RoleNameExists),
        ErrorKey::InvalidPermission => admin::render(lang, AdminError::InvalidPermission),
        ErrorKey::RolePermissionLocked => admin::render(lang, AdminError::RolePermissionLocked),
        ErrorKey::SystemRoleCannotBeDeleted => {
            admin::render(lang, AdminError::SystemRoleCannotBeDeleted)
        }
        ErrorKey::CannotRemoveOwnRoleAccess => {
            admin::render(lang, AdminError::CannotRemoveOwnRoleAccess)
        }
        ErrorKey::SuperadminOnly => admin::render(lang, AdminError::SuperadminOnly),
        ErrorKey::LastSuperadmin => admin::render(lang, AdminError::LastSuperadmin),
        ErrorKey::AdminAppointmentRestricted => {
            admin::render(lang, AdminError::AdminAppointmentRestricted)
        }
        ErrorKey::RankTooLow => admin::render(lang, AdminError::RankTooLow),
        ErrorKey::CannotGrantUnheldPermission => {
            admin::render(lang, AdminError::CannotGrantUnheldPermission)
        }
        ErrorKey::AppointNextLevelOnly => admin::render(lang, AdminError::AppointNextLevelOnly),
        ErrorKey::PresidentAcknowledgementRequired => {
            admin::render(lang, AdminError::PresidentAcknowledgementRequired)
        }
        ErrorKey::SettingsSuperadminOnly => admin::render(lang, AdminError::SettingsSuperadminOnly),
        ErrorKey::FareBatchEmpty => admin::render(lang, AdminError::FareBatchEmpty),
        ErrorKey::FareBatchTooLarge => admin::render(lang, AdminError::FareBatchTooLarge),
        ErrorKey::FareUnknownArea => admin::render(lang, AdminError::FareUnknownArea),
        ErrorKey::FareDuplicateConflict => admin::render(lang, AdminError::FareDuplicateConflict),

        ErrorKey::InvalidCredentials => auth::render(lang, AuthError::InvalidCredentials),
        ErrorKey::VerificationExpired => auth::render(lang, AuthError::VerificationExpired),
        ErrorKey::InvalidCode => auth::render(lang, AuthError::InvalidCode),
        ErrorKey::CodeAlreadyUsed => auth::render(lang, AuthError::CodeAlreadyUsed),
        ErrorKey::Invalid2FaSecret => auth::render(lang, AuthError::Invalid2FaSecret),
        ErrorKey::TwoFactorAlreadyEnabled => auth::render(lang, AuthError::TwoFactorAlreadyEnabled),
        ErrorKey::TwoFactorNotConfigured => auth::render(lang, AuthError::TwoFactorNotConfigured),
        ErrorKey::StartSetupFirst => auth::render(lang, AuthError::StartSetupFirst),
        ErrorKey::PasswordTooShort => auth::render(lang, AuthError::PasswordTooShort),
        ErrorKey::CurrentPasswordIncorrect => {
            auth::render(lang, AuthError::CurrentPasswordIncorrect)
        }
        ErrorKey::InviteExpired => auth::render(lang, AuthError::InviteExpired),
        ErrorKey::InviteAlreadyUsed => auth::render(lang, AuthError::InviteAlreadyUsed),
        ErrorKey::RecoveryLinkInvalid => auth::render(lang, AuthError::RecoveryLinkInvalid),
        ErrorKey::PasskeyFailed => auth::render(lang, AuthError::PasskeyFailed),
        ErrorKey::PasskeyRegistrationFailed(detail) => {
            auth::render(lang, AuthError::PasskeyRegistrationFailed(detail))
        }
        ErrorKey::ChallengeMismatch => auth::render(lang, AuthError::ChallengeMismatch),
        ErrorKey::NameRequired => auth::render(lang, AuthError::NameRequired),
        ErrorKey::NameTooLong => auth::render(lang, AuthError::NameTooLong),
        ErrorKey::PhoneInvalid => auth::render(lang, AuthError::PhoneInvalid),
        ErrorKey::RecentSignInRequired => auth::render(lang, AuthError::RecentSignInRequired),
        ErrorKey::EmailAlreadyRegistered => auth::render(lang, AuthError::EmailAlreadyRegistered),
        ErrorKey::AuthDisabled => auth::render(lang, AuthError::AuthDisabled),
        ErrorKey::AccountPending => auth::render(lang, AuthError::AccountPending),
        ErrorKey::TermsRequired => auth::render(lang, AuthError::TermsRequired),
        ErrorKey::SessionLocked => auth::render(lang, AuthError::SessionLocked),
        ErrorKey::UnlockMethodRequired => auth::render(lang, AuthError::UnlockMethodRequired),
        ErrorKey::TurnstileRequired => auth::render(lang, AuthError::TurnstileRequired),
        ErrorKey::TurnstileFailed => auth::render(lang, AuthError::TurnstileFailed),

        ErrorKey::EventNameRequired => events::render(lang, EventsError::EventNameRequired),
        ErrorKey::InvalidEventType => events::render(lang, EventsError::InvalidEventType),
        ErrorKey::SelectVisibleGroup => events::render(lang, EventsError::SelectVisibleGroup),
        ErrorKey::InvalidEventTime => events::render(lang, EventsError::InvalidEventTime),
        ErrorKey::InvalidSignupDeadline => events::render(lang, EventsError::InvalidSignupDeadline),
        ErrorKey::DeadlineAfterEvent => events::render(lang, EventsError::DeadlineAfterEvent),
        ErrorKey::EventNotOpen => events::render(lang, EventsError::EventNotOpen),
        ErrorKey::SignupClosed => events::render(lang, EventsError::SignupClosed),
        ErrorKey::DriverRegistrationRequired => {
            events::render(lang, EventsError::DriverRegistrationRequired)
        }
        ErrorKey::OnlyOrganiserCanCreate => {
            events::render(lang, EventsError::OnlyOrganiserCanCreate)
        }
        ErrorKey::OnlyOrganiserCanEdit => events::render(lang, EventsError::OnlyOrganiserCanEdit),
        ErrorKey::OnlyOrganiserCanCancel => {
            events::render(lang, EventsError::OnlyOrganiserCanCancel)
        }
        ErrorKey::OnlyOrganiserCanPerformAction => {
            events::render(lang, EventsError::OnlyOrganiserCanPerformAction)
        }
        ErrorKey::OnlyOrganiserCanViewSignups => {
            events::render(lang, EventsError::OnlyOrganiserCanViewSignups)
        }
        ErrorKey::OnlyOrganiserCanRecordAttendance => {
            events::render(lang, EventsError::OnlyOrganiserCanRecordAttendance)
        }
        ErrorKey::AttendanceModuleDisabled => {
            events::render(lang, EventsError::AttendanceModuleDisabled)
        }
        ErrorKey::InvalidCheckinCode => events::render(lang, EventsError::InvalidCheckinCode),
        ErrorKey::CheckinOutsideWindow => events::render(lang, EventsError::CheckinOutsideWindow),
        ErrorKey::CheckinCodeAmbiguous => events::render(lang, EventsError::CheckinCodeAmbiguous),
        ErrorKey::MembersOnlyAttendance => events::render(lang, EventsError::MembersOnlyAttendance),
        ErrorKey::EventNotOpenToGroup => events::render(lang, EventsError::EventNotOpenToGroup),
        ErrorKey::JoinAsRequired => events::render(lang, EventsError::JoinAsRequired),
        ErrorKey::JoinAsNotAllowed => events::render(lang, EventsError::JoinAsNotAllowed),
        ErrorKey::InvalidParentGroup => events::render(lang, EventsError::InvalidParentGroup),
        ErrorKey::EventRideNeedsSignup => events::render(lang, EventsError::EventRideNeedsSignup),
        ErrorKey::EventDriveNeedsSignup => events::render(lang, EventsError::EventDriveNeedsSignup),
        ErrorKey::DuplicateEventRide => events::render(lang, EventsError::DuplicateEventRide),
        ErrorKey::DriverOutsideGroup => events::render(lang, EventsError::DriverOutsideGroup),
        ErrorKey::VisitorLinkInvalid => events::render(lang, EventsError::VisitorLinkInvalid),
        ErrorKey::VisitorNotOpenYet => events::render(lang, EventsError::VisitorNotOpenYet),
        ErrorKey::VisitorLinkUsed => events::render(lang, EventsError::VisitorLinkUsed),
        ErrorKey::VisitorJourneyEnded => events::render(lang, EventsError::VisitorJourneyEnded),
        ErrorKey::VisitorNameRequired => events::render(lang, EventsError::VisitorNameRequired),
        ErrorKey::OnlyGoingCanInviteVisitor => {
            events::render(lang, EventsError::OnlyGoingCanInviteVisitor)
        }
        ErrorKey::InvalidPassCode => events::render(lang, EventsError::InvalidPassCode),
        ErrorKey::CarePassClosed => events::render(lang, EventsError::CarePassClosed),
        ErrorKey::CarePassNotOpenYet => events::render(lang, EventsError::CarePassNotOpenYet),
        ErrorKey::MileageAuthorisationRestricted => {
            events::render(lang, EventsError::MileageAuthorisationRestricted)
        }
        ErrorKey::EventNotDraft => events::render(lang, EventsError::EventNotDraft),
        ErrorKey::EventCannotHideWithSignups => {
            events::render(lang, EventsError::EventCannotHideWithSignups)
        }

        ErrorKey::InvalidPickupTime => rides::render(lang, RidesError::InvalidPickupTime),
        ErrorKey::PickupTimeInPast => rides::render(lang, RidesError::PickupTimeInPast),
        ErrorKey::SeatsRange => rides::render(lang, RidesError::SeatsRange),
        ErrorKey::AddressAndDestinationRequired => {
            rides::render(lang, RidesError::AddressAndDestinationRequired)
        }
        ErrorKey::PickupAreaRequired => rides::render(lang, RidesError::PickupAreaRequired),
        ErrorKey::RiderNameRequired => rides::render(lang, RidesError::RiderNameRequired),
        ErrorKey::CannotViewEvent => rides::render(lang, RidesError::CannotViewEvent),
        ErrorKey::CannotPickOwnRide => rides::render(lang, RidesError::CannotPickOwnRide),
        ErrorKey::FillVehicleDetailsFirst => {
            rides::render(lang, RidesError::FillVehicleDetailsFirst)
        }
        ErrorKey::NotEnoughSeatsSlot { load, cap } => rides::render(
            lang,
            RidesError::NotEnoughSeatsSlot {
                load: *load,
                cap: *cap,
            },
        ),
        ErrorKey::RidePickedByAnotherDriver => {
            rides::render(lang, RidesError::RidePickedByAnotherDriver)
        }
        ErrorKey::OnlyPassengerCanRespond => {
            rides::render(lang, RidesError::OnlyPassengerCanRespond)
        }
        ErrorKey::TripAlreadyCompleted => rides::render(lang, RidesError::TripAlreadyCompleted),
        ErrorKey::StartTripFirst => rides::render(lang, RidesError::StartTripFirst),
        ErrorKey::PassengerAlreadyPickedUp => {
            rides::render(lang, RidesError::PassengerAlreadyPickedUp)
        }
        ErrorKey::NoShowTooEarly => rides::render(lang, RidesError::NoShowTooEarly),
        ErrorKey::InvalidLocation => rides::render(lang, RidesError::InvalidLocation),
        ErrorKey::MaxLiveDriversReached { live, limit } => rides::render(
            lang,
            RidesError::MaxLiveDriversReached {
                live: *live,
                limit: *limit,
            },
        ),
        ErrorKey::RideStatusChanged => rides::render(lang, RidesError::RideStatusChanged),
        ErrorKey::GroupChatClosed => rides::render(lang, RidesError::GroupChatClosed),
        ErrorKey::GroupChatNotApproved => rides::render(lang, RidesError::GroupChatNotApproved),
        ErrorKey::OnlyTripMembersCanViewChat => {
            rides::render(lang, RidesError::OnlyTripMembersCanViewChat)
        }
        ErrorKey::InvalidMessageLength => rides::render(lang, RidesError::InvalidMessageLength),
        ErrorKey::InvalidReaction => rides::render(lang, RidesError::InvalidReaction),
        ErrorKey::NoDriverYet => rides::render(lang, RidesError::NoDriverYet),
        ErrorKey::DriverNotFound => rides::render(lang, RidesError::DriverNotFound),
        ErrorKey::NotEnoughDriverSeats { load, cap } => rides::render(
            lang,
            RidesError::NotEnoughDriverSeats {
                load: *load,
                cap: *cap,
            },
        ),
        ErrorKey::NotEnoughDriverSeatsReassign { load, cap } => rides::render(
            lang,
            RidesError::NotEnoughDriverSeatsReassign {
                load: *load,
                cap: *cap,
            },
        ),
        ErrorKey::NotRideDriver => rides::render(lang, RidesError::NotRideDriver),
        ErrorKey::NotRidePassenger => rides::render(lang, RidesError::NotRidePassenger),
        ErrorKey::CannotViewLocation => rides::render(lang, RidesError::CannotViewLocation),
        ErrorKey::CannotViewRide => rides::render(lang, RidesError::CannotViewRide),
        ErrorKey::OnlyVolunteerDriversCanSet => {
            rides::render(lang, RidesError::OnlyVolunteerDriversCanSet)
        }
        ErrorKey::SpareSeatsRange => rides::render(lang, RidesError::SpareSeatsRange),
        ErrorKey::VehicleModelAndPlateRequired => {
            rides::render(lang, RidesError::VehicleModelAndPlateRequired)
        }
        ErrorKey::VehicleSeatsRange => rides::render(lang, RidesError::VehicleSeatsRange),
        ErrorKey::MemberNotVolunteerDriver => {
            rides::render(lang, RidesError::MemberNotVolunteerDriver)
        }
        ErrorKey::DriverCannotBePassenger => {
            rides::render(lang, RidesError::DriverCannotBePassenger)
        }
        ErrorKey::OnlyPassengerOrStaffCanCancel => {
            rides::render(lang, RidesError::OnlyPassengerOrStaffCanCancel)
        }
        ErrorKey::OnlyVolunteerDriversCanView => {
            rides::render(lang, RidesError::OnlyVolunteerDriversCanView)
        }
        ErrorKey::OnlyVolunteerDriversCanChoose => {
            rides::render(lang, RidesError::OnlyVolunteerDriversCanChoose)
        }
        ErrorKey::OnlyTrafficLeadCanApprove => {
            rides::render(lang, RidesError::OnlyTrafficLeadCanApprove)
        }
        ErrorKey::PlanDraftNotFound => rides::render(lang, RidesError::PlanDraftNotFound),
        ErrorKey::DraftLockedByOther { name } => {
            rides::render(lang, RidesError::DraftLockedByOther { name: name.clone() })
        }
        ErrorKey::DraftNotEditable => rides::render(lang, RidesError::DraftNotEditable),
        ErrorKey::RideDraftNotFound => rides::render(lang, RidesError::RideDraftNotFound),

        ErrorKey::SettlementNameRequired => {
            settlements::render(lang, SettlementsError::SettlementNameRequired)
        }
        ErrorKey::AtLeastOneExpenseItem => {
            settlements::render(lang, SettlementsError::AtLeastOneExpenseItem)
        }
        ErrorKey::ExpenseItemNegativeOrBlank => {
            settlements::render(lang, SettlementsError::ExpenseItemNegativeOrBlank)
        }
        ErrorKey::AmountCannotBeNegative => {
            settlements::render(lang, SettlementsError::AmountCannotBeNegative)
        }
        ErrorKey::SubsidyCannotBeNegative => {
            settlements::render(lang, SettlementsError::SubsidyCannotBeNegative)
        }
        ErrorKey::SubsidyExceedsTotal => {
            settlements::render(lang, SettlementsError::SubsidyExceedsTotal)
        }
        ErrorKey::SelectParticipants => {
            settlements::render(lang, SettlementsError::SelectParticipants)
        }
        ErrorKey::DuplicateParticipant => {
            settlements::render(lang, SettlementsError::DuplicateParticipant)
        }
        ErrorKey::InvalidSplitMode => settlements::render(lang, SettlementsError::InvalidSplitMode),
        ErrorKey::MemberAmountRequired => {
            settlements::render(lang, SettlementsError::MemberAmountRequired)
        }
        ErrorKey::CustomSplitMismatch { sum, total } => {
            settlements::render(lang, SettlementsError::CustomSplitMismatch { sum, total })
        }
        ErrorKey::OnlyDraftsCanBeDeleted => {
            settlements::render(lang, SettlementsError::OnlyDraftsCanBeDeleted)
        }
        ErrorKey::OnlyOrganiserCanEditBill => {
            settlements::render(lang, SettlementsError::OnlyOrganiserCanEditBill)
        }
        ErrorKey::OnlyOrganiserCanPublish => {
            settlements::render(lang, SettlementsError::OnlyOrganiserCanPublish)
        }
        ErrorKey::OnlyOrganiserCanCloseBill => {
            settlements::render(lang, SettlementsError::OnlyOrganiserCanCloseBill)
        }
        ErrorKey::SettlementAlreadyPublished => {
            settlements::render(lang, SettlementsError::SettlementAlreadyPublished)
        }
        ErrorKey::RevisionReasonRequired => {
            settlements::render(lang, SettlementsError::RevisionReasonRequired)
        }
        ErrorKey::UnattendedMembers(count) => {
            settlements::render(lang, SettlementsError::UnattendedMembers(*count))
        }
        ErrorKey::OpenPaymentsRemain(count) => {
            settlements::render(lang, SettlementsError::OpenPaymentsRemain(*count))
        }
        ErrorKey::PaidMembersCannotBeRemoved(count) => {
            settlements::render(lang, SettlementsError::PaidMembersCannotBeRemoved(*count))
        }
        ErrorKey::PayeeDetailsRequired => {
            settlements::render(lang, SettlementsError::PayeeDetailsRequired)
        }
        ErrorKey::OnlyPayeeCanVerify => {
            settlements::render(lang, SettlementsError::OnlyPayeeCanVerify)
        }
        ErrorKey::ProofOnlyForOwnShare => {
            settlements::render(lang, SettlementsError::ProofOnlyForOwnShare)
        }
        ErrorKey::ProofNotNeededNow => {
            settlements::render(lang, SettlementsError::ProofNotNeededNow)
        }
        ErrorKey::UploadReceiptFirst => {
            settlements::render(lang, SettlementsError::UploadReceiptFirst)
        }
        ErrorKey::InvalidProofDocument => {
            settlements::render(lang, SettlementsError::InvalidProofDocument)
        }
        ErrorKey::TxnRefRequired => settlements::render(lang, SettlementsError::TxnRefRequired),
        ErrorKey::DuplicateTxnOrProof => {
            settlements::render(lang, SettlementsError::DuplicateTxnOrProof)
        }
        ErrorKey::NeedMoreInfoReasonRequired => {
            settlements::render(lang, SettlementsError::NeedMoreInfoReasonRequired)
        }
        ErrorKey::NotOnCostSplitList => {
            settlements::render(lang, SettlementsError::NotOnCostSplitList)
        }
        ErrorKey::BillClosedCannotEdit => {
            settlements::render(lang, SettlementsError::BillClosedCannotEdit)
        }
        ErrorKey::OnlyOrganiserCanInitiateCostSplit => {
            settlements::render(lang, SettlementsError::OnlyOrganiserCanInitiateCostSplit)
        }

        ErrorKey::QuestionAndDetailsRequired => {
            community::render(lang, CommunityError::QuestionAndDetailsRequired)
        }
        ErrorKey::RequestAlreadyClosed => {
            community::render(lang, CommunityError::RequestAlreadyClosed)
        }
        ErrorKey::ReplyContentRequired => {
            community::render(lang, CommunityError::ReplyContentRequired)
        }
        ErrorKey::LanguageInvalid => community::render(lang, CommunityError::LanguageInvalid),
        ErrorKey::TitleAndContentRequired => {
            community::render(lang, CommunityError::TitleAndContentRequired)
        }
        ErrorKey::GuideTitleRequired => community::render(lang, CommunityError::GuideTitleRequired),
        ErrorKey::GuideAudienceInvalid => {
            community::render(lang, CommunityError::GuideAudienceInvalid)
        }
        ErrorKey::HelpAttachmentInvalid => {
            community::render(lang, CommunityError::HelpAttachmentInvalid)
        }
        ErrorKey::GuideSlugRequired => community::render(lang, CommunityError::GuideSlugRequired),
        ErrorKey::GuideSlugExists => community::render(lang, CommunityError::GuideSlugExists),
        ErrorKey::GuideLinkInvalid => community::render(lang, CommunityError::GuideLinkInvalid),
        ErrorKey::GuideStepInvalid => community::render(lang, CommunityError::GuideStepInvalid),
        ErrorKey::OnlyLinkOwnRides => community::render(lang, CommunityError::OnlyLinkOwnRides),
        ErrorKey::AmountRangeInvalid => community::render(lang, CommunityError::AmountRangeInvalid),
        ErrorKey::PayeeNameAndPhoneRequired => {
            community::render(lang, CommunityError::PayeeNameAndPhoneRequired)
        }
        ErrorKey::ReceiptRequired => community::render(lang, CommunityError::ReceiptRequired),
        ErrorKey::OnlyReturnedReimbCanBeResubmitted => {
            community::render(lang, CommunityError::OnlyReturnedReimbCanBeResubmitted)
        }
        ErrorKey::ReimbAlreadyReviewed => {
            community::render(lang, CommunityError::ReimbAlreadyReviewed)
        }
        ErrorKey::CannotReviewOwnRequest => {
            community::render(lang, CommunityError::CannotReviewOwnRequest)
        }
        ErrorKey::OnlyApprovedReimbCanBePaid => {
            community::render(lang, CommunityError::OnlyApprovedReimbCanBePaid)
        }
        ErrorKey::CannotPayOwnRequest => {
            community::render(lang, CommunityError::CannotPayOwnRequest)
        }
        ErrorKey::CannotPayOwnApproval => {
            community::render(lang, CommunityError::CannotPayOwnApproval)
        }
        ErrorKey::VisitorReimbNeedsDriver => {
            community::render(lang, CommunityError::VisitorReimbNeedsDriver)
        }
        ErrorKey::VisitorReimbAlreadyLive => {
            community::render(lang, CommunityError::VisitorReimbAlreadyLive)
        }
        ErrorKey::AmountMustBePositive => {
            community::render(lang, CommunityError::AmountMustBePositive)
        }
        ErrorKey::VisitorHasReimbursement => {
            community::render(lang, CommunityError::VisitorHasReimbursement)
        }
        ErrorKey::ReimbursementAlreadyClaimed => {
            community::render(lang, CommunityError::ReimbursementAlreadyClaimed)
        }
        ErrorKey::EventRequiredForReimbursement => {
            community::render(lang, CommunityError::EventRequiredForReimbursement)
        }
        ErrorKey::EventReimbursementNeedsSignup => {
            community::render(lang, CommunityError::EventReimbursementNeedsSignup)
        }
        ErrorKey::PayeeVerificationRequired => {
            community::render(lang, CommunityError::PayeeVerificationRequired)
        }
        ErrorKey::ReimbursementProcessed => {
            community::render(lang, CommunityError::ReimbursementProcessed)
        }
        ErrorKey::ReasonRequired => community::render(lang, CommunityError::ReasonRequired),
        ErrorKey::MonthFormatInvalid => community::render(lang, CommunityError::MonthFormatInvalid),
        ErrorKey::BatchEmpty => community::render(lang, CommunityError::BatchEmpty),
        ErrorKey::BatchAlreadyPaid => community::render(lang, CommunityError::BatchAlreadyPaid),

        ErrorKey::NoFileReceived => files::render(lang, FilesError::NoFileReceived),
        ErrorKey::FileTooLarge => files::render(lang, FilesError::FileTooLarge),
        ErrorKey::InvalidFileType => files::render(lang, FilesError::InvalidFileType),
        ErrorKey::UploadFailed => files::render(lang, FilesError::UploadFailed),
        ErrorKey::UploadQuotaReached => files::render(lang, FilesError::UploadQuotaReached),
        ErrorKey::InvalidReceipt => files::render(lang, FilesError::InvalidReceipt),
        ErrorKey::InvalidPhoto => files::render(lang, FilesError::InvalidPhoto),
        ErrorKey::CannotViewFile => files::render(lang, FilesError::CannotViewFile),

        ErrorKey::FamilyActiveOnly => family::render(lang, FamilyError::ActiveOnly),
        ErrorKey::AlreadyInFamily => family::render(lang, FamilyError::AlreadyInFamily),
        ErrorKey::FamilyRequestPending => family::render(lang, FamilyError::RequestPending),
        ErrorKey::FamilyOwnPhone => family::render(lang, FamilyError::OwnPhone),
        ErrorKey::FamilyPhoneAmbiguous => family::render(lang, FamilyError::PhoneAmbiguous),
        ErrorKey::FamilyPhoneUnavailable => family::render(lang, FamilyError::PhoneUnavailable),
        ErrorKey::FamilyFull { max } => family::render(lang, FamilyError::Full(*max)),
        ErrorKey::FamilyOwnerCannotLeave => family::render(lang, FamilyError::OwnerCannotLeave),
    }
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::i18n::keys::ErrorKey;
    use crate::i18n::Lang;

    /// Generates both `samples()` and a compile-time exhaustive variant check so a new
    /// `ErrorKey` variant can never be omitted from the test suite.
    macro_rules! all_error_samples {
        ($($pat:pat => $sample:expr),+ $(,)?) => {{
            fn assert_exhaustive(k: &ErrorKey) {
                match k {
                    $($pat => (),)+
                }
            }
            let list = vec![$($sample),+];
            for item in &list {
                assert_exhaustive(item);
            }
            list
        }};
    }

    fn samples() -> Vec<ErrorKey> {
        all_error_samples! {
            ErrorKey::NotFound => ErrorKey::NotFound,
            ErrorKey::Unauthorized => ErrorKey::Unauthorized,
            ErrorKey::Forbidden => ErrorKey::Forbidden,
            ErrorKey::ServerError => ErrorKey::ServerError,
            ErrorKey::InvalidAction => ErrorKey::InvalidAction,
            ErrorKey::InvalidStatus => ErrorKey::InvalidStatus,
            ErrorKey::InvalidDate => ErrorKey::InvalidDate,
            ErrorKey::NoPermission => ErrorKey::NoPermission,
            ErrorKey::MfaRequired => ErrorKey::MfaRequired,
            ErrorKey::RateLimited => ErrorKey::RateLimited,

            ErrorKey::CannotDisableSelf => ErrorKey::CannotDisableSelf,
            ErrorKey::CannotGrantSelfFinance => ErrorKey::CannotGrantSelfFinance,
            ErrorKey::SecondAdminRequired => ErrorKey::SecondAdminRequired,
            ErrorKey::CannotDeleteAllMembersGroup => ErrorKey::CannotDeleteAllMembersGroup,
            ErrorKey::GroupNameRequired => ErrorKey::GroupNameRequired,
            ErrorKey::GroupNameExists => ErrorKey::GroupNameExists,
            ErrorKey::AreaNameRequired => ErrorKey::AreaNameRequired,
            ErrorKey::AreaExists => ErrorKey::AreaExists,
            ErrorKey::AreaPhotoLimit => ErrorKey::AreaPhotoLimit,
            ErrorKey::AreaPhotoCaptionTooLong => ErrorKey::AreaPhotoCaptionTooLong,
            ErrorKey::RequestAlreadyHandled => ErrorKey::RequestAlreadyHandled,
            ErrorKey::EmailInvalid => ErrorKey::EmailInvalid,
            ErrorKey::UnknownRole => ErrorKey::UnknownRole,
            ErrorKey::RoleNameRequired => ErrorKey::RoleNameRequired,
            ErrorKey::RoleNameExists => ErrorKey::RoleNameExists,
            ErrorKey::InvalidPermission => ErrorKey::InvalidPermission,
            ErrorKey::RolePermissionLocked => ErrorKey::RolePermissionLocked,
            ErrorKey::SystemRoleCannotBeDeleted => ErrorKey::SystemRoleCannotBeDeleted,
            ErrorKey::CannotRemoveOwnRoleAccess => ErrorKey::CannotRemoveOwnRoleAccess,
            ErrorKey::SuperadminOnly => ErrorKey::SuperadminOnly,
            ErrorKey::LastSuperadmin => ErrorKey::LastSuperadmin,
            ErrorKey::AdminAppointmentRestricted => ErrorKey::AdminAppointmentRestricted,
            ErrorKey::RankTooLow => ErrorKey::RankTooLow,
            ErrorKey::CannotGrantUnheldPermission => ErrorKey::CannotGrantUnheldPermission,
            ErrorKey::AppointNextLevelOnly => ErrorKey::AppointNextLevelOnly,
            ErrorKey::PresidentAcknowledgementRequired => ErrorKey::PresidentAcknowledgementRequired,
            ErrorKey::SettingsSuperadminOnly => ErrorKey::SettingsSuperadminOnly,
            ErrorKey::FareBatchEmpty => ErrorKey::FareBatchEmpty,
            ErrorKey::FareBatchTooLarge => ErrorKey::FareBatchTooLarge,
            ErrorKey::FareUnknownArea => ErrorKey::FareUnknownArea,
            ErrorKey::FareDuplicateConflict => ErrorKey::FareDuplicateConflict,

            ErrorKey::InvalidCredentials => ErrorKey::InvalidCredentials,
            ErrorKey::VerificationExpired => ErrorKey::VerificationExpired,
            ErrorKey::InvalidCode => ErrorKey::InvalidCode,
            ErrorKey::CodeAlreadyUsed => ErrorKey::CodeAlreadyUsed,
            ErrorKey::Invalid2FaSecret => ErrorKey::Invalid2FaSecret,
            ErrorKey::TwoFactorAlreadyEnabled => ErrorKey::TwoFactorAlreadyEnabled,
            ErrorKey::TwoFactorNotConfigured => ErrorKey::TwoFactorNotConfigured,
            ErrorKey::StartSetupFirst => ErrorKey::StartSetupFirst,
            ErrorKey::PasswordTooShort => ErrorKey::PasswordTooShort,
            ErrorKey::CurrentPasswordIncorrect => ErrorKey::CurrentPasswordIncorrect,
            ErrorKey::InviteExpired => ErrorKey::InviteExpired,
            ErrorKey::InviteAlreadyUsed => ErrorKey::InviteAlreadyUsed,
            ErrorKey::RecoveryLinkInvalid => ErrorKey::RecoveryLinkInvalid,
            ErrorKey::PasskeyFailed => ErrorKey::PasskeyFailed,
            ErrorKey::PasskeyRegistrationFailed(_) => ErrorKey::PasskeyRegistrationFailed("x".into()),
            ErrorKey::ChallengeMismatch => ErrorKey::ChallengeMismatch,
            ErrorKey::NameRequired => ErrorKey::NameRequired,
            ErrorKey::NameTooLong => ErrorKey::NameTooLong,
            ErrorKey::PhoneInvalid => ErrorKey::PhoneInvalid,
            ErrorKey::RecentSignInRequired => ErrorKey::RecentSignInRequired,
            ErrorKey::EmailAlreadyRegistered => ErrorKey::EmailAlreadyRegistered,
            ErrorKey::AuthDisabled => ErrorKey::AuthDisabled,
            ErrorKey::AccountPending => ErrorKey::AccountPending,
            ErrorKey::TermsRequired => ErrorKey::TermsRequired,
            ErrorKey::SessionLocked => ErrorKey::SessionLocked,
            ErrorKey::UnlockMethodRequired => ErrorKey::UnlockMethodRequired,
            ErrorKey::TurnstileRequired => ErrorKey::TurnstileRequired,
            ErrorKey::TurnstileFailed => ErrorKey::TurnstileFailed,

            ErrorKey::EventNameRequired => ErrorKey::EventNameRequired,
            ErrorKey::InvalidEventType => ErrorKey::InvalidEventType,
            ErrorKey::SelectVisibleGroup => ErrorKey::SelectVisibleGroup,
            ErrorKey::InvalidEventTime => ErrorKey::InvalidEventTime,
            ErrorKey::InvalidSignupDeadline => ErrorKey::InvalidSignupDeadline,
            ErrorKey::DeadlineAfterEvent => ErrorKey::DeadlineAfterEvent,
            ErrorKey::EventNotOpen => ErrorKey::EventNotOpen,
            ErrorKey::SignupClosed => ErrorKey::SignupClosed,
            ErrorKey::DriverRegistrationRequired => ErrorKey::DriverRegistrationRequired,
            ErrorKey::OnlyOrganiserCanCreate => ErrorKey::OnlyOrganiserCanCreate,
            ErrorKey::OnlyOrganiserCanEdit => ErrorKey::OnlyOrganiserCanEdit,
            ErrorKey::OnlyOrganiserCanCancel => ErrorKey::OnlyOrganiserCanCancel,
            ErrorKey::OnlyOrganiserCanPerformAction => ErrorKey::OnlyOrganiserCanPerformAction,
            ErrorKey::OnlyOrganiserCanViewSignups => ErrorKey::OnlyOrganiserCanViewSignups,
            ErrorKey::OnlyOrganiserCanRecordAttendance => ErrorKey::OnlyOrganiserCanRecordAttendance,
            ErrorKey::AttendanceModuleDisabled => ErrorKey::AttendanceModuleDisabled,
            ErrorKey::InvalidCheckinCode => ErrorKey::InvalidCheckinCode,
            ErrorKey::CheckinOutsideWindow => ErrorKey::CheckinOutsideWindow,
            ErrorKey::CheckinCodeAmbiguous => ErrorKey::CheckinCodeAmbiguous,
            ErrorKey::MembersOnlyAttendance => ErrorKey::MembersOnlyAttendance,
            ErrorKey::EventNotOpenToGroup => ErrorKey::EventNotOpenToGroup,
            ErrorKey::JoinAsRequired => ErrorKey::JoinAsRequired,
            ErrorKey::JoinAsNotAllowed => ErrorKey::JoinAsNotAllowed,
            ErrorKey::InvalidParentGroup => ErrorKey::InvalidParentGroup,
            ErrorKey::EventRideNeedsSignup => ErrorKey::EventRideNeedsSignup,
            ErrorKey::EventDriveNeedsSignup => ErrorKey::EventDriveNeedsSignup,
            ErrorKey::DuplicateEventRide => ErrorKey::DuplicateEventRide,
            ErrorKey::DriverOutsideGroup => ErrorKey::DriverOutsideGroup,
            ErrorKey::VisitorLinkInvalid => ErrorKey::VisitorLinkInvalid,
            ErrorKey::VisitorNotOpenYet => ErrorKey::VisitorNotOpenYet,
            ErrorKey::VisitorLinkUsed => ErrorKey::VisitorLinkUsed,
            ErrorKey::VisitorJourneyEnded => ErrorKey::VisitorJourneyEnded,
            ErrorKey::VisitorNameRequired => ErrorKey::VisitorNameRequired,
            ErrorKey::OnlyGoingCanInviteVisitor => ErrorKey::OnlyGoingCanInviteVisitor,
            ErrorKey::InvalidPassCode => ErrorKey::InvalidPassCode,
            ErrorKey::CarePassClosed => ErrorKey::CarePassClosed,
            ErrorKey::CarePassNotOpenYet => ErrorKey::CarePassNotOpenYet,
            ErrorKey::MileageAuthorisationRestricted => ErrorKey::MileageAuthorisationRestricted,
            ErrorKey::EventNotDraft => ErrorKey::EventNotDraft,
            ErrorKey::EventCannotHideWithSignups => ErrorKey::EventCannotHideWithSignups,

            ErrorKey::InvalidPickupTime => ErrorKey::InvalidPickupTime,
            ErrorKey::PickupTimeInPast => ErrorKey::PickupTimeInPast,
            ErrorKey::SeatsRange => ErrorKey::SeatsRange,
            ErrorKey::AddressAndDestinationRequired => ErrorKey::AddressAndDestinationRequired,
            ErrorKey::PickupAreaRequired => ErrorKey::PickupAreaRequired,
            ErrorKey::RiderNameRequired => ErrorKey::RiderNameRequired,
            ErrorKey::CannotViewEvent => ErrorKey::CannotViewEvent,
            ErrorKey::CannotPickOwnRide => ErrorKey::CannotPickOwnRide,
            ErrorKey::FillVehicleDetailsFirst => ErrorKey::FillVehicleDetailsFirst,
            ErrorKey::NotEnoughSeatsSlot { .. } => ErrorKey::NotEnoughSeatsSlot { load: 1, cap: 1 },
            ErrorKey::RidePickedByAnotherDriver => ErrorKey::RidePickedByAnotherDriver,
            ErrorKey::OnlyPassengerCanRespond => ErrorKey::OnlyPassengerCanRespond,
            ErrorKey::TripAlreadyCompleted => ErrorKey::TripAlreadyCompleted,
            ErrorKey::StartTripFirst => ErrorKey::StartTripFirst,
            ErrorKey::PassengerAlreadyPickedUp => ErrorKey::PassengerAlreadyPickedUp,
            ErrorKey::NoShowTooEarly => ErrorKey::NoShowTooEarly,
            ErrorKey::InvalidLocation => ErrorKey::InvalidLocation,
            ErrorKey::MaxLiveDriversReached { .. } => ErrorKey::MaxLiveDriversReached { live: 1, limit: 1 },
            ErrorKey::RideStatusChanged => ErrorKey::RideStatusChanged,
            ErrorKey::GroupChatClosed => ErrorKey::GroupChatClosed,
            ErrorKey::GroupChatNotApproved => ErrorKey::GroupChatNotApproved,
            ErrorKey::OnlyTripMembersCanViewChat => ErrorKey::OnlyTripMembersCanViewChat,
            ErrorKey::InvalidMessageLength => ErrorKey::InvalidMessageLength,
            ErrorKey::InvalidReaction => ErrorKey::InvalidReaction,
            ErrorKey::NoDriverYet => ErrorKey::NoDriverYet,
            ErrorKey::DriverNotFound => ErrorKey::DriverNotFound,
            ErrorKey::NotEnoughDriverSeats { .. } => ErrorKey::NotEnoughDriverSeats { load: 1, cap: 1 },
            ErrorKey::NotEnoughDriverSeatsReassign { .. } => ErrorKey::NotEnoughDriverSeatsReassign { load: 1, cap: 1 },
            ErrorKey::NotRideDriver => ErrorKey::NotRideDriver,
            ErrorKey::NotRidePassenger => ErrorKey::NotRidePassenger,
            ErrorKey::CannotViewLocation => ErrorKey::CannotViewLocation,
            ErrorKey::CannotViewRide => ErrorKey::CannotViewRide,
            ErrorKey::OnlyVolunteerDriversCanSet => ErrorKey::OnlyVolunteerDriversCanSet,
            ErrorKey::SpareSeatsRange => ErrorKey::SpareSeatsRange,
            ErrorKey::VehicleModelAndPlateRequired => ErrorKey::VehicleModelAndPlateRequired,
            ErrorKey::VehicleSeatsRange => ErrorKey::VehicleSeatsRange,
            ErrorKey::MemberNotVolunteerDriver => ErrorKey::MemberNotVolunteerDriver,
            ErrorKey::DriverCannotBePassenger => ErrorKey::DriverCannotBePassenger,
            ErrorKey::OnlyPassengerOrStaffCanCancel => ErrorKey::OnlyPassengerOrStaffCanCancel,
            ErrorKey::OnlyVolunteerDriversCanView => ErrorKey::OnlyVolunteerDriversCanView,
            ErrorKey::OnlyVolunteerDriversCanChoose => ErrorKey::OnlyVolunteerDriversCanChoose,
            ErrorKey::OnlyTrafficLeadCanApprove => ErrorKey::OnlyTrafficLeadCanApprove,
            ErrorKey::PlanDraftNotFound => ErrorKey::PlanDraftNotFound,
            ErrorKey::DraftLockedByOther { .. } => {
                ErrorKey::DraftLockedByOther { name: "x".into() }
            },
            ErrorKey::DraftNotEditable => ErrorKey::DraftNotEditable,
            ErrorKey::RideDraftNotFound => ErrorKey::RideDraftNotFound,

            ErrorKey::SettlementNameRequired => ErrorKey::SettlementNameRequired,
            ErrorKey::AtLeastOneExpenseItem => ErrorKey::AtLeastOneExpenseItem,
            ErrorKey::ExpenseItemNegativeOrBlank => ErrorKey::ExpenseItemNegativeOrBlank,
            ErrorKey::AmountCannotBeNegative => ErrorKey::AmountCannotBeNegative,
            ErrorKey::SubsidyCannotBeNegative => ErrorKey::SubsidyCannotBeNegative,
            ErrorKey::SubsidyExceedsTotal => ErrorKey::SubsidyExceedsTotal,
            ErrorKey::SelectParticipants => ErrorKey::SelectParticipants,
            ErrorKey::DuplicateParticipant => ErrorKey::DuplicateParticipant,
            ErrorKey::InvalidSplitMode => ErrorKey::InvalidSplitMode,
            ErrorKey::MemberAmountRequired => ErrorKey::MemberAmountRequired,
            ErrorKey::CustomSplitMismatch { .. } => ErrorKey::CustomSplitMismatch {
                sum: "1".into(),
                total: "1".into(),
            },
            ErrorKey::OnlyDraftsCanBeDeleted => ErrorKey::OnlyDraftsCanBeDeleted,
            ErrorKey::OnlyOrganiserCanEditBill => ErrorKey::OnlyOrganiserCanEditBill,
            ErrorKey::OnlyOrganiserCanPublish => ErrorKey::OnlyOrganiserCanPublish,
            ErrorKey::OnlyOrganiserCanCloseBill => ErrorKey::OnlyOrganiserCanCloseBill,
            ErrorKey::SettlementAlreadyPublished => ErrorKey::SettlementAlreadyPublished,
            ErrorKey::RevisionReasonRequired => ErrorKey::RevisionReasonRequired,
            ErrorKey::UnattendedMembers(_) => ErrorKey::UnattendedMembers(1),
            ErrorKey::OpenPaymentsRemain(_) => ErrorKey::OpenPaymentsRemain(1),
            ErrorKey::PaidMembersCannotBeRemoved(_) => ErrorKey::PaidMembersCannotBeRemoved(1),
            ErrorKey::PayeeDetailsRequired => ErrorKey::PayeeDetailsRequired,
            ErrorKey::OnlyPayeeCanVerify => ErrorKey::OnlyPayeeCanVerify,
            ErrorKey::ProofOnlyForOwnShare => ErrorKey::ProofOnlyForOwnShare,
            ErrorKey::ProofNotNeededNow => ErrorKey::ProofNotNeededNow,
            ErrorKey::UploadReceiptFirst => ErrorKey::UploadReceiptFirst,
            ErrorKey::InvalidProofDocument => ErrorKey::InvalidProofDocument,
            ErrorKey::TxnRefRequired => ErrorKey::TxnRefRequired,
            ErrorKey::DuplicateTxnOrProof => ErrorKey::DuplicateTxnOrProof,
            ErrorKey::NeedMoreInfoReasonRequired => ErrorKey::NeedMoreInfoReasonRequired,
            ErrorKey::NotOnCostSplitList => ErrorKey::NotOnCostSplitList,
            ErrorKey::BillClosedCannotEdit => ErrorKey::BillClosedCannotEdit,
            ErrorKey::OnlyOrganiserCanInitiateCostSplit => ErrorKey::OnlyOrganiserCanInitiateCostSplit,

            ErrorKey::QuestionAndDetailsRequired => ErrorKey::QuestionAndDetailsRequired,
            ErrorKey::RequestAlreadyClosed => ErrorKey::RequestAlreadyClosed,
            ErrorKey::ReplyContentRequired => ErrorKey::ReplyContentRequired,
            ErrorKey::LanguageInvalid => ErrorKey::LanguageInvalid,
            ErrorKey::TitleAndContentRequired => ErrorKey::TitleAndContentRequired,
            ErrorKey::GuideTitleRequired => ErrorKey::GuideTitleRequired,
            ErrorKey::GuideAudienceInvalid => ErrorKey::GuideAudienceInvalid,
            ErrorKey::HelpAttachmentInvalid => ErrorKey::HelpAttachmentInvalid,
            ErrorKey::GuideSlugRequired => ErrorKey::GuideSlugRequired,
            ErrorKey::GuideSlugExists => ErrorKey::GuideSlugExists,
            ErrorKey::GuideLinkInvalid => ErrorKey::GuideLinkInvalid,
            ErrorKey::GuideStepInvalid => ErrorKey::GuideStepInvalid,
            ErrorKey::OnlyLinkOwnRides => ErrorKey::OnlyLinkOwnRides,
            ErrorKey::AmountRangeInvalid => ErrorKey::AmountRangeInvalid,
            ErrorKey::PayeeNameAndPhoneRequired => ErrorKey::PayeeNameAndPhoneRequired,
            ErrorKey::ReceiptRequired => ErrorKey::ReceiptRequired,
            ErrorKey::OnlyReturnedReimbCanBeResubmitted => ErrorKey::OnlyReturnedReimbCanBeResubmitted,
            ErrorKey::ReimbAlreadyReviewed => ErrorKey::ReimbAlreadyReviewed,
            ErrorKey::CannotReviewOwnRequest => ErrorKey::CannotReviewOwnRequest,
            ErrorKey::OnlyApprovedReimbCanBePaid => ErrorKey::OnlyApprovedReimbCanBePaid,
            ErrorKey::CannotPayOwnRequest => ErrorKey::CannotPayOwnRequest,
            ErrorKey::CannotPayOwnApproval => ErrorKey::CannotPayOwnApproval,
            ErrorKey::ReimbursementAlreadyClaimed => ErrorKey::ReimbursementAlreadyClaimed,
            ErrorKey::EventRequiredForReimbursement => ErrorKey::EventRequiredForReimbursement,
            ErrorKey::EventReimbursementNeedsSignup => ErrorKey::EventReimbursementNeedsSignup,
            ErrorKey::PayeeVerificationRequired => ErrorKey::PayeeVerificationRequired,
            ErrorKey::ReimbursementProcessed => ErrorKey::ReimbursementProcessed,
            ErrorKey::ReasonRequired => ErrorKey::ReasonRequired,
            ErrorKey::MonthFormatInvalid => ErrorKey::MonthFormatInvalid,
            ErrorKey::BatchEmpty => ErrorKey::BatchEmpty,
            ErrorKey::BatchAlreadyPaid => ErrorKey::BatchAlreadyPaid,
            ErrorKey::VisitorReimbNeedsDriver => ErrorKey::VisitorReimbNeedsDriver,
            ErrorKey::VisitorReimbAlreadyLive => ErrorKey::VisitorReimbAlreadyLive,
            ErrorKey::VisitorHasReimbursement => ErrorKey::VisitorHasReimbursement,
            ErrorKey::AmountMustBePositive => ErrorKey::AmountMustBePositive,

            ErrorKey::NoFileReceived => ErrorKey::NoFileReceived,
            ErrorKey::FileTooLarge => ErrorKey::FileTooLarge,
            ErrorKey::InvalidFileType => ErrorKey::InvalidFileType,
            ErrorKey::UploadFailed => ErrorKey::UploadFailed,
            ErrorKey::UploadQuotaReached => ErrorKey::UploadQuotaReached,
            ErrorKey::InvalidReceipt => ErrorKey::InvalidReceipt,
            ErrorKey::InvalidPhoto => ErrorKey::InvalidPhoto,
            ErrorKey::CannotViewFile => ErrorKey::CannotViewFile,

            ErrorKey::FamilyActiveOnly => ErrorKey::FamilyActiveOnly,
            ErrorKey::AlreadyInFamily => ErrorKey::AlreadyInFamily,
            ErrorKey::FamilyRequestPending => ErrorKey::FamilyRequestPending,
            ErrorKey::FamilyOwnPhone => ErrorKey::FamilyOwnPhone,
            ErrorKey::FamilyPhoneAmbiguous => ErrorKey::FamilyPhoneAmbiguous,
            ErrorKey::FamilyPhoneUnavailable => ErrorKey::FamilyPhoneUnavailable,
            ErrorKey::FamilyFull { .. } => ErrorKey::FamilyFull { max: 8 },
            ErrorKey::FamilyOwnerCannotLeave => ErrorKey::FamilyOwnerCannotLeave,
        }
    }

    #[test]
    fn every_error_has_a_unique_code_and_both_languages() {
        let dynamic = [
            "PasskeyRegistrationFailed",
            "NotEnoughSeatsSlot",
            "MaxLiveDriversReached",
            "NotEnoughDriverSeats",
            "NotEnoughDriverSeatsReassign",
            "DraftLockedByOther",
            "CustomSplitMismatch",
            "UnattendedMembers",
            "OpenPaymentsRemain",
            "PaidMembersCannotBeRemoved",
            "FamilyFull",
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
