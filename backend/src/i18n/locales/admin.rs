//! Admin errors. Both languages live next to each other.

use super::pair;
use crate::i18n::{keys::ErrorKey, Lang};

pub fn render(lang: Lang, key: &ErrorKey) -> String {
    match key {
        ErrorKey::CannotDisableSelf => pair(
            lang,
            "不能停用自己或移除自己管理角色的权限",
            "You cannot disable yourself or remove your own right to manage roles",
        ),
        ErrorKey::CannotGrantSelfFinance => pair(
            lang,
            "不能自行授予财务权限，请由另一位管理员操作",
            "You cannot grant yourself finance permissions; another admin must do it",
        ),
        ErrorKey::UnknownRole => pair(
            lang,
            "所选角色已不存在，请刷新后重试",
            "A selected role no longer exists; please reload and try again",
        ),
        ErrorKey::RoleNameRequired => pair(
            lang,
            "请填写角色名称（最多 40 个字）",
            "Please enter a role name (up to 40 characters)",
        ),
        ErrorKey::RoleNameExists => pair(lang, "角色名称已存在", "This role name already exists"),
        ErrorKey::InvalidPermission => pair(
            lang,
            "权限项目无效，请刷新后重试",
            "Unknown permission; please reload and try again",
        ),
        ErrorKey::RolePermissionLocked => pair(
            lang,
            "管理员角色必须保留成员与角色管理权限",
            "The admin role must keep member and role management",
        ),
        ErrorKey::SystemRoleCannotBeDeleted => pair(
            lang,
            "内置角色不能删除，可改为调整其权限",
            "Built-in roles cannot be deleted; change their permissions instead",
        ),
        ErrorKey::CannotRemoveOwnRoleAccess => pair(
            lang,
            "不能移除自己管理角色的权限",
            "You cannot remove your own right to manage roles",
        ),
        ErrorKey::SecondAdminRequired => pair(
            lang,
            "恢复管理职务账号需要另一位管理员确认",
            "Recovering a privileged account requires a second, different admin to confirm",
        ),
        ErrorKey::CannotDeleteAllMembersGroup => pair(
            lang,
            "不能删除“全体会员”群体",
            "The \"All members\" group cannot be deleted",
        ),
        ErrorKey::GroupNameRequired => pair(lang, "请填写群体名称", "Please enter a group name"),
        ErrorKey::GroupNameExists => pair(lang, "群体名称已存在", "This group name already exists"),
        ErrorKey::AreaNameRequired => pair(lang, "请填写地区名称", "Please enter an area name"),
        ErrorKey::AreaExists => pair(lang, "地区已存在", "This area already exists"),
        ErrorKey::RequestAlreadyHandled => pair(
            lang,
            "此申请已处理",
            "This request has already been handled",
        ),
        ErrorKey::EmailInvalid => pair(lang, "电邮格式不正确", "Invalid email address"),
        ErrorKey::EmailAlreadyMember => {
            pair(lang, "此电邮已是会员", "This email is already a member")
        }
        other => panic!("{} is not an admin error", other.code()),
    }
}
