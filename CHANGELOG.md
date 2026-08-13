# Changelog

## [1.5.0] - 2026-08-13

### Changed
- Upgraded to rmcp 3.1.2 and raised the minimum supported Rust version to 1.94.1.
- Added MCP 2026-07-28 stateless request handling while retaining MCP 2025-11-25 initialization compatibility.

### Added
- Per-request identity and protocol metadata, on-demand discovery/cache hints, and the configured Tasks and sealed MRTR approval policies.

## [1.4.0] - 2026-05-27

### Added — Backend Integrations
- `sync_google_calendar` — bidirectional sync with Google Calendar API v3
- `sync_outlook` — sync with Microsoft Graph API (Outlook/365)
- `sync_calcom` — pull bookings from Cal.com API v2
- `sync_calendly` — pull scheduled events from Calendly API

## [1.3.0] - 2026-05-27

### Added — Advanced Scheduling
- `waitlist_join` — join queue when slot is full
- `round_robin_assign` — auto-assign to least-busy resource from pool
- `reminder_set` — set reminder N minutes before (push/email/sms)
- `slot_hold` — tentative reservation with expiry
- `group_booking` — capacity-based bookings (classes, events)
- `ics_export` — export as iCalendar for Google/Outlook/Apple
- `booking_link_create` — shareable self-service booking URL

## [1.2.0] - 2026-05-27

### Added — 22-Country Holidays
- Expanded holiday coverage: UG, TZ, ET, RW, ZA, EG, GH, FR, JP, CN, BR, AU, CA, MX
- Now covers 22 countries with cultural and religious holidays

## [1.1.0] - 2026-05-27

### Added — Timezone & Cultural Awareness
- `holidays_list` — public holidays for 8+ countries
- `timezone_convert` — convert between 40+ IANA timezones
- `find_overlap` — find common working hours across timezones
- `work_week_set` — cultural work patterns (Mon-Fri, Sun-Thu, Mon-Sat)
- `buffer_set` — travel/setup time between bookings
- `blackout_set` — company-wide no-booking periods

## [1.0.0] - 2026-05-27

### Added — Core Scheduling
- `resource_create` / `resource_list` — schedulable resources
- `booking_create` / `booking_cancel` / `booking_reschedule` / `booking_list` — appointments with conflict detection
- `availability_check` / `find_slot` — free time windows
- `shift_create` / `shift_update` / `shift_list` — employee shifts
- `time_off_request` / `time_off_decide` / `time_off_list` — leave management
- `schedule_summary` — daily overview
