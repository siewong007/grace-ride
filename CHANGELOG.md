# Changelog

## [Unreleased]
- Rides and driving now start from an event. **My rides** and **Driving** are no longer in the menu or tab bar; a member signs up for an event first, then asks for a ride (or, for drivers, offers to drive) on that event's page, where a driver sees only that event's requests. The server enforces it: `POST /rides` needs an `event_id` the member is going to, and a driver may claim a ride only for an event they are going to (`EVENT_RIDE_NEEDS_SIGNUP`, `EVENT_DRIVE_NEEDS_SIGNUP`). Ride history and driving history (schedule, vehicle and areas) live under **Profile → Related pages**. The non-event recurring ride and standing driver schedule screens are removed; recurring events cover that need.
- Profile page refresh: a larger avatar card at the top, and each section row shows a one-line summary (phone, push and reminder settings, passkeys and 2FA). Tap the avatar to change it: upload or take a photo and crop it to a circle (it's resized in the browser, which also drops its location data), or show your initial or an icon on a colour you choose. Photos can now be removed. The chosen avatar also appears in the top bar, on the lock screen, on the ride driver card, and in the admin member list.
- Trip chat: members can react to a message with an emoji (👍 ❤️ 😂 😮 😢 🙏; one per person, tap again to remove), and their own messages show WhatsApp-style ticks: one tick sent, two ticks delivered to every other trip member's app, two coloured ticks once everyone has read it. Staff viewing a chat don't count towards delivered or read.
- Announcements can be sent to chosen member groups (and their sub-groups). Only those members see them on Home and get the notification; leaving every group unticked still reaches everyone. Admins can change an announcement's groups when editing it.

## [0.1.0] - 2026-11-18
- Initial release.
- Refactored repository structure to improve maintainability.
- Added husky pre-commit hooks for linting.
- Added documentation for refactor process.
