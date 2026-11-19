# Family groups

An approved member can create one family, name it, and add people by phone number so those people can get their own Grace Ride login. The family is a list of names. It does not change rides, bookings, church groups, or admin approval.

## Decisions

- A phone that matches one active or pending member sends that member an in-app request. They accept or decline.
- A phone that matches nobody creates a one-time invite link. The owner copies the link and sends it. The app does not text an unknown number.
- The new person types their own name and email. The phone stays the number the owner entered. The terms box starts unchecked.
- Accepting the link creates a pending account and puts them on the family list in the same transaction. Rides, events, and every other feature stay locked until an admin sets the account to active and assigns roles and church groups.
- One family per person. One owner, the member who created it. Only the owner adds, removes, renames, or deletes. Anyone else on the list can leave. The owner closes the family by deleting it.
- A pending member can view the list, accept or decline a request, and leave. They cannot create a family or add someone.
- A family holds at most 8 people. Open requests and unused, unexpired links count toward the 8.

## What a member sees

Family lives on Profile and at `/me/family`. The notification for an in-app request links to `/me/family`.

The screen shows the family name and each member's name. A member whose account is still `pending` is marked waiting for approval. Phones and emails are not on the member list.

The owner also sees open requests (the member's name) and unused links (the phone they typed, the expiry, copy, and cancel). Another member sees the list and Leave. Accept and decline appear when this person has an incoming request.

Creating, renaming, adding, removing, and deleting are owner actions. A member who is not approved does not see a way to create a family.

Deleting the family removes every member, request, and unused invite. Accounts stay. Removing one person, or that person leaving, takes them off the list and leaves their account as it is.

Church groups, rides, bookings, and proxy riders stay as they are. Joining a family does not approve an account and does not add a church group.

## Phone matching

`check_phone` still requires a non-empty value of 10–20 characters. The comparison key is the digits of that value. One leading `0` is replaced with `60`, so `012-345 6789`, `+60 12-345 6789`, and `60123456789` are the same key. No other rewriting is done. The text the owner typed is what is stored on the invite and, after acceptance, on the new user's `phone`.

Only users with status `active` or `pending` are candidates. Disabled rows are ignored when an active or pending row matches.

Checks run in this order. The first match wins.

1. The phone fails `check_phone` or is empty: `PHONE_INVALID`.
2. The caller is not the owner: `NO_PERMISSION`. The caller is the owner but not `active`: `FAMILY_ACTIVE_ONLY`.
3. The key is the owner's own phone: `FAMILY_OWN_PHONE`.
4. This family already has an open request, or an unused unexpired link, for that key: return that same request or link. This is success, including when the family is already at 8.
5. Two or more active or pending users share the key: `FAMILY_PHONE_AMBIGUOUS`.
6. The one active or pending user is already in a family: `ALREADY_IN_FAMILY`.
7. The single matching user already has an open request, or another family has an unused unexpired link for that key: `FAMILY_REQUEST_PENDING`. The other family's link still blocks the number when no account matches it. Nothing new is created.
8. No active or pending user, and at least one disabled user has the key: `FAMILY_PHONE_UNAVAILABLE`.
9. Members plus this family's open requests plus this family's unused, unexpired invites is already 8: `FAMILY_FULL`.
10. Exactly one active or pending user remains: create an in-app request. `notify` writes it, so an active member also gets their existing push or WhatsApp alert if they already opted in.
11. No matching user: create a family invite link.

A member with a blank phone does not match. The owner gets a link. If that person later accepts with an email that is already registered, the existing `EMAIL_ALREADY_REGISTERED` error applies and the link stays unused.

## Invite link

A family invite is a row in `invites` with `kind = 'family'`. It uses the ordinary invite rules: the raw token is stored so the owner can copy the link again, it expires in 3 days (`in_days(3)`), and it works once. Recovery links stay hashed and are unchanged. `INVITE_BY_TOKEN` already matches every non-recovery kind on the raw token.

`GET /api/invites/:token` for this kind returns `kind`, `family_name`, and the locked `phone`. `email` and `name` are empty. It does not return members, the owner, or any other account.

The invite page (`/invite/:token`) shows the family name and the phone. The phone field is filled and not editable. Name and email start empty. The terms box starts unchecked. The passkey step after acceptance is unchanged.

`POST /api/invites/:token/accept` still takes `name`, `email`, `phone`, and `terms`. For a family invite the server ignores the submitted phone and writes the phone stored on the invite. Name, email, and terms are checked the same way as an admin invite. Roles and church groups stay empty. Status is `pending`.

Acceptance runs in one transaction:

1. The invite is unused and unexpired, and the family still exists.
2. Count members, open requests, and unused unexpired invites except this one, then add 1 for the person accepting. If that total is over 8, stop with `FAMILY_FULL`. A link that fit when it was created still fits, because this invite leaves the unused set as the person joins.
3. The pending user is inserted and added to `family_members`, and the invite is marked used.

The copied link is `{public_url}/invite/{token}`, the same shape as an admin invite.

If any check fails, or the email is already registered, the transaction rolls back and the link stays unused. On success the existing admin notification fires (`Page::AdminMembers`, `Act::Update`, link `/admin/members?status=pending`).

## Records

Migration `backend/migrations/0015_families.sql`.

`families`: `id`, `name` (non-empty, at most 60 characters, enforced by `check_name`), `owner_id` references `users(id)` on delete cascade, `created_at`. Names are not unique.

`family_members`: `family_id` and `user_id`, both cascade on delete, `created_at`, primary key `(family_id, user_id)`, unique `user_id`.

`family_requests`: `id`, `family_id` cascade, `user_id` cascade, `created_at`, unique `user_id`. One open request per person.

`invites` gains nullable `family_id` (cascade on delete) and `phone text not null default ''`. The kind check becomes `kind IN ('invite', 'recovery', 'family')` and keeps the name `chk_invites_kind`. A new check `chk_invites_family` requires a family row to have a `family_id` and a phone of at least 10 characters, and every other kind to have `family_id` null. `schema_baseline` lists `chk_invites_family` next to `chk_invites_kind`.

Deleting a family deletes its members, requests, and every invite row tied to it, used or not. Deleting a user who owns a family deletes that family. Accounts created through a family invite are normal user rows and are not deleted with the family. `invite.accept` stays in the audit log.

## API

All family routes are under `/api/me/family`. `pending_profile_only` allows `/me/family` and everything under it, so a pending member can open this section and nothing else new. These routes are not added to the session-lock bypass. Responses keep `{error, code, params?}`.

`GET /api/me/family` returns:

- `family`: `{id, name, owner_id}` or null
- `members`: `{user_id, name, status}` for the caller's family, or `[]`
- `requests`: owner only, `{id, user_id, name}`; otherwise `[]`
- `invites`: owner only, `{id, phone, expires_at, url}`; otherwise `[]`
- `incoming`: `{id, family_name, owner_name}` when the caller has an open request and is not in a family; otherwise null

| Action | Route | Who |
|---|---|---|
| Create | `POST /api/me/family` `{name}` | Active member who is not in a family. A pending caller gets `FAMILY_ACTIVE_ONLY` before the membership check. |
| Rename | `PATCH /api/me/family` `{name}` | Owner |
| Delete | `DELETE /api/me/family` | Owner |
| Leave | `POST /api/me/family/leave` | A member who is not the owner |
| Add by phone | `POST /api/me/family/people` `{phone}` | Owner. Returns `{kind: "request", id, name}` or `{kind: "invite", id, phone, expires_at, url}` |
| Remove member | `DELETE /api/me/family/members/:user_id` | Owner, and not themselves |
| Cancel request | `DELETE /api/me/family/requests/:id` | Owner |
| Revoke link | `DELETE /api/me/family/invites/:id` | Owner. Sets the invite expired |
| Accept request | `POST /api/me/family/requests/:id/accept` | The requested user |
| Decline request | `POST /api/me/family/requests/:id/decline` | The requested user |

Accept and decline are conditional updates. A request that is already gone returns `REQUEST_ALREADY_HANDLED`. Accept inserts `family_members` and deletes the request in one transaction. If the person is already in a family or the family is full, the transaction rolls back and that request row stays.

Audit actions, with ids and no phone number in the detail text: `family.create`, `family.rename`, `family.delete`, `family.request`, `family.request.accept`, `family.request.decline`, `family.request.cancel`, `family.invite`, `family.invite.revoke`, `family.member.remove`, `family.leave`. Accepting a link also writes the existing `invite.accept` audit.

## Admin

Staff who can view members (`Page::AdminMembers`, `Act::View`) receive `family_name` on each user in the members list, or null. The pending row and the approve dialog show that name as read-only text. Callers who use the same list without member-view permission do not receive `family_name`. Approval still sets status, roles, and church groups. It does not edit the family.

## Errors

New keys live in a `family` group in the Rust catalog and in `frontend` `errors` locales, both Chinese and English. `ERROR_GROUPS` in `frontend/src/api.ts` includes `family`.

| Code | When | English | 中文 |
|---|---|---|---|
| `FAMILY_ACTIVE_ONLY` | A pending member creates a family or adds someone | You can create a family or add someone after an admin approves your account | 账号通过审核后才能建立或邀请家人 |
| `ALREADY_IN_FAMILY` | The owner or the matched person is already in a family | Already in a family | 已经在一个家庭里了 |
| `FAMILY_REQUEST_PENDING` | Another family already has an open request or unused link for this number | This number already has an open request from another family | 这个号码已有另一个家庭的待处理邀请 |
| `FAMILY_OWN_PHONE` | The owner typed their own number | That is your own number | 这是你自己的号码 |
| `FAMILY_PHONE_AMBIGUOUS` | More than one active or pending account has this key | This number matches more than one member. Ask an admin | 这个号码对应多位会员，请联系管理员 |
| `FAMILY_PHONE_UNAVAILABLE` | The only accounts for this key are disabled | This number cannot be added. Ask an admin | 这个号码无法加入，请联系管理员 |
| `FAMILY_FULL` | The cap of 8 is reached. The key carries `max`, and the frontend string interpolates `{max}` the same way other counted errors do | A family can have at most {max} people | 一个家庭最多 {max} 人 |
| `FAMILY_OWNER_CANNOT_LEAVE` | The owner tries to leave, or to remove themselves | The owner deletes the family instead of leaving | 户主请删除整个家庭，而不能退出 |

These existing codes stay in use: `NAME_REQUIRED`, `NAME_TOO_LONG`, `PHONE_INVALID`, `NO_PERMISSION`, `NOT_FOUND`, `INVITE_EXPIRED`, `INVITE_ALREADY_USED`, `TERMS_REQUIRED`, `EMAIL_INVALID`, `EMAIL_ALREADY_REGISTERED`, `REQUEST_ALREADY_HANDLED`.

There is no new rate limiter. The cap of 8 bounds how many pending accounts one owner can create.

## Privacy

The family screen shows names and account status only. The owner sees a phone only on an unused link they created. The public invite preview shows the family name and that locked phone. Audit text does not include the phone. `family_name` on the admin list is limited to staff who can already view members.

## Documentation

Update `docs/TECHNICAL.md` in three places: a short Family row in the module inventory, the membership workflow (a family invite is a second way onto the pending list, and `/api/me/family` is the pending-account exception), and the new tables in the database section. Add a short "My family" paragraph to `docs/USER_MANUAL.md` on the profile: create a family, add a phone, copy the link, and wait for admin approval. Do not describe ride sharing.

## Tests

Unit, no database: the phone key. `012-345 6789`, `+60 12-345 6789`, and `60123456789` match. An empty value and a value outside 10–20 characters still fail `check_phone`. The existing catalog test covers the new codes in both languages.

API, in the PostgreSQL harness:

- An active member creates a family and is its only member and its owner. A pending member gets `FAMILY_ACTIVE_ONLY`. A second family for the same person gets `ALREADY_IN_FAMILY`.
- A number that matches one member creates a request and a notification, and does not create a user. Accept joins. Decline does not. A second family requesting that person while the request is open gets `FAMILY_REQUEST_PENDING`.
- A number that matches nobody returns a link. Preview shows the family name and the locked phone and no members. Accepting with terms creates a pending user with empty roles and no church groups, adds them to the family, and notifies admins. Accepting without terms leaves the link unused. The same number again returns that link.
- Own number, someone already in a family, two accounts on one key, and a disabled-only key create neither a request nor a link.
- The 9th counted person returns `FAMILY_FULL`.
- Remove and leave take the person off the list and keep the account. The owner cannot leave. Delete clears members, requests, and unused links, and keeps the accounts.
- A pending member can load the family and leave. A ride route still returns `ACCOUNT_PENDING`.
- A non-owner cannot rename, add, or remove.
- Accepting a link after the family was deleted does not create a user. The invite row is gone with the family, so the token is not found.
- Church group rows are unchanged by every case above.
- A non-staff caller of the members list does not receive `family_name`.

Frontend:

- Profile covers creating a family, the member list, the waiting-for-approval mark, and a member row with no phone.
- The invite page covers a family link with the phone filled and locked, the terms box unchecked, and name and email empty.

During implementation, check the same path in the browser: create a family, add a number, copy the link, accept it, and see the pending member on the list.

## Out of scope

- Sending SMS or WhatsApp to a number that is not already a member.
- Letting family members see or book one another's rides.
- Treating a family as a church group, event audience, or announcement audience.
- Letting an admin edit the family, or letting a non-owner invite someone.
- More than one family per person, or more than 8 counted people.
- Changing proxy riders or the caregiver safe-arrival link.
