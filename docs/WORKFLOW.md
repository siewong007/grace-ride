# Workflow

Current as of 14 November 2026. Feature scope and source ownership are in [Features](FEATURES.md).

## Membership and account access

1. An authorized administrator creates an invitation. The invitee enters their details and accepts the terms.
2. Staff approves membership and assigns appropriate groups and roles. Pending accounts have restricted access.
3. Members use configured sign-in methods; staff permissions require a passkey or MFA session. Profile manages language, sign-in methods, TOTP backup codes and auto-lock preference.
4. Account recovery is reviewed by staff; privileged recovery needs two administrators. See [Security](SECURITY.md) for the exact controls.

## Request and deliver a ride

1. A member enters pickup time, area/address, destination, seats and assistance needs. Proxy booking can name a family member or elder and save their details for reuse; the signed-in member manages the booking.
2. A volunteer driver claims a request. The passenger accepts or declines, then transport staff approves it after capacity checks.
3. Approval exposes permitted contact/pickup information and opens trip chat. Before approval, drivers receive the approximate pickup area.
4. Passenger readiness, driver delay and arrival-at-gate actions communicate pickup changes. The driver starts the ride, records pickup and completes it.
5. Location sharing is limited to active trips. Route suggestions and navigation links support pickup ordering. Chat stops accepting messages when the trip is finished.
6. Driver withdrawal flags the request for attention. Staff can reassign it for the new driver's confirmation, return it, cancel it or record alternative transport. No-show is a separate outcome.

For an event, transport staff can use the planning board to review passengers and drivers, pin choices, request auto-arrange suggestions, and apply a reviewed batch. Suggestions do not silently assign drivers; the backend rechecks stale plans transactionally.

## Activities and money

- Organizers create group-visible events. Members sign up, offer driving or request transport. Capacity can create a waitlist; withdrawals promote waiting members and cancel linked rides as applicable. Organizers record attendance.
- Organizers prepare activity expenses, subsidy and equal/custom attendee shares. Publish the bill; members submit external-payment proof; the payee verifies it. Revisions notify affected members, and disputed shares need resolution.
- Substitute-transport claims go through transport review and manual finance payout recording. Completed volunteer rides can generate rate-based reimbursement records. These records do not transfer money.
- Members review notifications, help threads and monthly reports; staff uses dashboards, reports, audit and settings according to permissions.

The Help centre has a Guides tab for everyone. Members see published member guides, while staff who can access any staff page see both member and staff guides. Help editors set each guide's audience and publication state (/help → Guides).

Only a superadmin can grant or remove the superadmin role. The last active superadmin cannot be demoted or disabled, and role changes require MFA. Recovery of a privileged account requires approval from two different admins.

## Development and documentation

1. Inspect the working tree and preserve unrelated edits. Read the canonical document for the module being changed and the relevant source/tests.
2. Distinguish a requirement or suggestion from an implemented feature. Preserve invite-only membership, voluntary rides, server-side permissions and sensitive-data restrictions.
3. Implement a bounded change and run the relevant [checks](TESTING.md). Record actual commands/results; never infer passing tests from historical reports.
4. Update the affected canonical document in the same change. Record a resolved defect in [Resolved issues](RESOLVED_ISSUES.md), including evidence and how to detect a regression.
5. Review the diff for stale links, duplicated claims and unrelated changes. Do not create another progress file, UI phase plan or timestamped documentation copy.
6. Commit, publish or deploy only within the task's authorization. Repository deployment behavior is described in [Deployment](DEPLOYMENT.md).

The previous Cline scheduler setup is retired as documentation. This repository audit does not establish that an external scheduler is installed, running or disabled.
