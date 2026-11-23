//! Admin errors. Both languages live next to each other.

use super::pair;
use crate::i18n::Lang;

pub enum AdminError {
    CannotDisableSelf,
    CannotGrantSelfFinance,
    SecondAdminRequired,
    CannotDeleteAllMembersGroup,
    GroupNameRequired,
    GroupNameExists,
    AreaNameRequired,
    AreaExists,
    AreaPhotoLimit,
    AreaPhotoCaptionTooLong,
    RequestAlreadyHandled,
    EmailInvalid,
    UnknownRole,
    RoleNameRequired,
    RoleNameExists,
    InvalidPermission,
    RolePermissionLocked,
    SystemRoleCannotBeDeleted,
    CannotRemoveOwnRoleAccess,
    SuperadminOnly,
    LastSuperadmin,
    AdminAppointmentRestricted,
    RankTooLow,
    CannotGrantUnheldPermission,
    AppointNextLevelOnly,
    PresidentAcknowledgementRequired,
    SettingsSuperadminOnly,
    FareBatchEmpty,
    FareBatchTooLarge,
    FareUnknownArea,
    FareDuplicateConflict,
}

pub fn render(lang: Lang, key: AdminError) -> String {
    match key {
        AdminError::CannotDisableSelf => pair(
            lang,
            "不能停用自己或移除自己管理角色的权限",
            "You cannot disable yourself or remove your own right to manage roles",
        ),
        AdminError::CannotGrantSelfFinance => pair(
            lang,
            "不能自行授予财务权限，请由另一位管理员操作",
            "You cannot grant yourself finance permissions; another admin must do it",
        ),
        AdminError::UnknownRole => pair(
            lang,
            "所选角色已不存在，请刷新后重试",
            "A selected role no longer exists; please reload and try again",
        ),
        AdminError::RoleNameRequired => pair(
            lang,
            "请填写角色名称（最多 40 个字）",
            "Please enter a role name (up to 40 characters)",
        ),
        AdminError::RoleNameExists => pair(lang, "角色名称已存在", "This role name already exists"),
        AdminError::InvalidPermission => pair(
            lang,
            "权限项目无效，请刷新后重试",
            "Unknown permission; please reload and try again",
        ),
        AdminError::RolePermissionLocked => pair(
            lang,
            "超级管理员角色固定拥有全部权限，不能修改",
            "The super admin role always holds every permission and cannot be changed",
        ),
        AdminError::SystemRoleCannotBeDeleted => pair(
            lang,
            "内置角色不能删除，可改为调整其权限",
            "Built-in roles cannot be deleted; change their permissions instead",
        ),
        AdminError::CannotRemoveOwnRoleAccess => pair(
            lang,
            "不能移除自己管理角色的权限",
            "You cannot remove your own right to manage roles",
        ),
        AdminError::SuperadminOnly => pair(
            lang,
            "只有超级管理员可以授予超级管理员角色或变更其账号",
            "Only a super admin can give the super admin role or change a super admin's account",
        ),
        AdminError::LastSuperadmin => pair(
            lang,
            "必须保留至少一位启用中的超级管理员",
            "There must always be one active super admin",
        ),
        AdminError::AdminAppointmentRestricted => pair(
            lang,
            "只有超级管理员、会长或副会长可以委任或卸任管理员",
            "Only the super admin, President or Vice-President can appoint or demote an administrator",
        ),
        AdminError::RankTooLow => pair(
            lang,
            "只能管理职级低于自己的成员和角色",
            "You can only manage members and roles ranked below you",
        ),
        AdminError::CannotGrantUnheldPermission => pair(
            lang,
            "不能授予自己不具备的权限",
            "You can't grant a permission you don't hold yourself",
        ),
        AdminError::AppointNextLevelOnly => pair(
            lang,
            "只能委任比自己低一级的职务",
            "You can only appoint the level directly below your own",
        ),
        AdminError::PresidentAcknowledgementRequired => pair(
            lang,
            "副会长将普通会员升为管理员前，必须先取得会长确认并勾选确认",
            "A Vice-President must confirm the President has acknowledged the appointment before making an ordinary member an administrator",
        ),
        AdminError::SettingsSuperadminOnly => pair(
            lang,
            "只有超级管理员可以授予系统设置权限",
            "Only the super admin can grant system settings access",
        ),
        AdminError::FareBatchEmpty => pair(
            lang,
            "请至少提供一条车费记录",
            "Provide at least one fare entry",
        ),
        AdminError::FareBatchTooLarge => pair(
            lang,
            "单次最多导入 10,000 条车费记录",
            "At most 10,000 fare entries can be imported at once",
        ),
        AdminError::FareUnknownArea => pair(
            lang,
            "导入包含未知地区，请先新增地区或勾选自动创建",
            "Some areas in the import do not exist; add them first or allow creating missing areas",
        ),
        AdminError::FareDuplicateConflict => pair(
            lang,
            "同一路线在导入文件中存在不同的车费金额",
            "The same route appears with conflicting fare amounts",
        ),
        AdminError::SecondAdminRequired => pair(
            lang,
            "恢复管理职务账号需要另一位管理员确认",
            "Recovering a privileged account requires a second, different admin to confirm",
        ),
        AdminError::CannotDeleteAllMembersGroup => pair(
            lang,
            "不能删除“全体会员”群体",
            "The \"All members\" group cannot be deleted",
        ),
        AdminError::GroupNameRequired => pair(lang, "请填写群体名称", "Please enter a group name"),
        AdminError::GroupNameExists => {
            pair(lang, "群体名称已存在", "This group name already exists")
        }
        AdminError::AreaNameRequired => pair(lang, "请填写地区名称", "Please enter an area name"),
        AdminError::AreaExists => pair(lang, "地区已存在", "This area already exists"),
        AdminError::AreaPhotoLimit => pair(
            lang,
            "每个地区最多 10 张照片，请先删除一张",
            "An area can have up to 10 photos; remove one first",
        ),
        AdminError::AreaPhotoCaptionTooLong => pair(
            lang,
            "照片说明不能超过 120 个字",
            "A photo caption can be up to 120 characters",
        ),
        AdminError::RequestAlreadyHandled => pair(
            lang,
            "此申请已处理",
            "This request has already been handled",
        ),
        AdminError::EmailInvalid => pair(lang, "电邮格式不正确", "Invalid email address"),
    }
}
