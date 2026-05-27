# Scheduling MCP Server

[![Crates.io](https://img.shields.io/crates/v/mcp-scheduling.svg)](https://crates.io/crates/mcp-scheduling)
[![Docs.rs](https://docs.rs/mcp-scheduling/badge.svg)](https://docs.rs/mcp-scheduling)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![ADK-Rust Enterprise](https://img.shields.io/badge/ADK--Rust-Enterprise-purple.svg)](https://enterprise.adk-rust.com)
[![Registry Ready](https://img.shields.io/badge/ADK_Registry-Ready-green.svg)](https://enterprise.adk-rust.com)

Global scheduling engine for [ADK-Rust Enterprise](https://enterprise.adk-rust.com) agents. Provides 32 MCP tools covering appointments, shifts, resource booking, availability, conflict detection, 22-country holidays, multi-timezone overlap, waitlists, round-robin assignment, group bookings, and sync with Google Calendar, Outlook, Cal.com, and Calendly.

## Architecture

```
┌────────────────────────────────────────────────────────────────────────────┐
│                        mcp-scheduling (32 tools)                            │
├──────────┬──────────┬──────────┬──────────┬──────────┬─────────────────────┤
│ Bookings │  Shifts  │ Cultural │ Advanced │  Export  │   Integrations      │
├──────────┼──────────┼──────────┼──────────┼──────────┼─────────────────────┤
│ Create   │ Schedule │ Holidays │ Waitlist │ ICS      │ Google Calendar     │
│ Cancel   │ Update   │ Timezone │ Round-   │ Booking  │ Microsoft Outlook   │
│ Reschedl │ List     │ Overlap  │  robin   │  links   │ Cal.com             │
│ Hold     │          │ Work wk  │ Reminder │          │ Calendly            │
│ Group    │          │ Buffer   │          │          │                     │
│          │          │ Blackout │          │          │                     │
└──────────┴──────────┴──────────┴──────────┴──────────┴─────────────────────┘
```

## Key Principles

- **Conflict-free** — rejects double-bookings automatically, respects time off and blackouts.
- **Globally aware** — 22-country holidays, 40+ timezones, cultural work weeks (Mon-Fri, Sun-Thu, Mon-Sat).
- **Multi-timezone meetings** — find overlapping working hours across Nairobi, NYC, and Singapore.
- **Feature parity** — matches Calendly + Cal.com (waitlists, round-robin, booking links, group bookings).
- **Backend sync** — bidirectional sync with Google Calendar, Outlook, Cal.com, Calendly.
- **Zero configuration** — starts immediately. Integrations activate when env vars are set.

## Tools (32)

### Resources (2)

| Tool | Description |
|------|-------------|
| `resource_create` | Create schedulable resource (person, room, equipment, vehicle) |
| `resource_list` | List all resources |

### Bookings (6)

| Tool | Description |
|------|-------------|
| `booking_create` | Create booking with automatic conflict detection |
| `booking_cancel` | Cancel a booking |
| `booking_reschedule` | Move to new time (checks conflicts at new time) |
| `booking_list` | List bookings for resource/date range |
| `slot_hold` | Tentatively hold a slot (expires in N minutes) |
| `group_booking` | Capacity-based booking (classes, events, group sessions) |

### Availability (2)

| Tool | Description |
|------|-------------|
| `availability_check` | Get free time slots for a resource on a date |
| `find_slot` | Find first available slot across multiple resources |

### Shifts (3)

| Tool | Description |
|------|-------------|
| `shift_create` | Schedule employee shift (checks time off) |
| `shift_update` | Update status (confirmed, started, completed, no_show) |
| `shift_list` | List shifts for resource/date range |

### Time Off (3)

| Tool | Description |
|------|-------------|
| `time_off_request` | Request leave (vacation, sick, personal) |
| `time_off_decide` | Approve or reject request |
| `time_off_list` | List time off for a resource |

### Timezone & Cultural (6)

| Tool | Description |
|------|-------------|
| `holidays_list` | Public holidays for 22 countries |
| `timezone_convert` | Convert between 40+ IANA timezones |
| `find_overlap` | Find common working hours across timezones |
| `work_week_set` | Set cultural work pattern (Mon-Fri, Sun-Thu, Mon-Sat) |
| `buffer_set` | Set travel/setup time between bookings |
| `blackout_set` | Block dates (company closures, maintenance) |

### Advanced (4)

| Tool | Description |
|------|-------------|
| `waitlist_join` | Join queue when slot is full |
| `round_robin_assign` | Auto-assign to least-busy resource |
| `reminder_set` | Set reminder N minutes before |
| `booking_link_create` | Generate shareable self-service booking URL |

### Export & Summary (2)

| Tool | Description |
|------|-------------|
| `ics_export` | Export as iCalendar (Google/Outlook/Apple import) |
| `schedule_summary` | Daily view (bookings + shifts + leave) |

### Backend Integrations (4)

| Tool | Backend | Env Var |
|------|---------|---------|
| `sync_google_calendar` | Google Calendar API v3 | `GOOGLE_CALENDAR_TOKEN` |
| `sync_outlook` | Microsoft Graph API | `MICROSOFT_GRAPH_TOKEN` |
| `sync_calcom` | Cal.com API v2 | `CALCOM_API_KEY` |
| `sync_calendly` | Calendly API | `CALENDLY_TOKEN` |

## Installation

```bash
cargo install mcp-scheduling
```

### Client Configuration

```json
{
  "mcpServers": {
    "scheduling": {
      "command": "mcp-scheduling",
      "env": {
        "GOOGLE_CALENDAR_TOKEN": "ya29.xxx (optional)",
        "CALCOM_API_KEY": "cal_xxx (optional)"
      }
    }
  }
}
```

Works with Claude Desktop, Kiro, Cursor, Windsurf, Codex, and any MCP client.

## Quick Start

### 1. Create resources

```json
{"name": "resource_create", "arguments": {"name": "Dr. Amina", "resource_type": "person", "timezone": "Africa/Nairobi"}}
{"name": "resource_create", "arguments": {"name": "Conference Room A", "resource_type": "room", "capacity": 12}}
```

### 2. Book an appointment

```json
{"name": "booking_create", "arguments": {"resource_id": "res_abc", "title": "Strategy Meeting", "start": "2026-06-15T09:00:00+03:00", "end": "2026-06-15T10:00:00+03:00", "booked_by": "james"}}
```

### 3. Find overlap for international meeting

```json
{"name": "find_overlap", "arguments": {"participants": [{"name": "James", "timezone": "Africa/Nairobi"}, {"name": "Sarah", "timezone": "America/New_York"}, {"name": "Wei", "timezone": "Asia/Singapore"}], "duration_minutes": 60, "date": "2026-06-15"}}
```

### 4. Round-robin assignment

```json
{"name": "round_robin_assign", "arguments": {"resource_ids": ["res_1", "res_2", "res_3"], "title": "Customer Call", "start": "2026-06-15T14:00:00Z", "end": "2026-06-15T14:30:00Z", "booked_by": "system"}}
```

### 5. Group booking (yoga class, 20 capacity)

```json
{"name": "group_booking", "arguments": {"resource_id": "res_studio", "title": "Morning Yoga", "start": "2026-06-15T07:00:00", "end": "2026-06-15T08:00:00", "capacity": 20, "attendees": ["alice", "bob", "carol"], "booked_by": "instructor"}}
```

### 6. Sync with Google Calendar

```json
{"name": "sync_google_calendar", "arguments": {"direction": "pull", "resource_id": "res_abc", "start": "2026-06-01T00:00:00Z", "end": "2026-06-30T23:59:59Z"}}
```

## Holiday Coverage (22 Countries)

| Region | Countries |
|--------|-----------|
| East Africa | 🇰🇪 Kenya, 🇺🇬 Uganda, 🇹🇿 Tanzania, 🇪🇹 Ethiopia, 🇷🇼 Rwanda |
| West/South Africa | 🇳🇬 Nigeria, 🇬🇭 Ghana, 🇿🇦 South Africa, 🇪🇬 Egypt |
| Europe | 🇬🇧 UK, 🇩🇪 Germany, 🇫🇷 France |
| Asia | 🇮🇳 India, 🇨🇳 China, 🇯🇵 Japan, 🇸🇬 Singapore |
| Middle East | 🇦🇪 UAE, 🇸🇦 Saudi Arabia (Islamic holidays) |
| Americas | 🇺🇸 US, 🇨🇦 Canada, 🇧🇷 Brazil, 🇲🇽 Mexico |
| Oceania | 🇦🇺 Australia |

## Timezone Support (40+)

| Region | Timezones |
|--------|-----------|
| Africa | Nairobi, Lagos, Cairo, Johannesburg, Addis Ababa, Kigali, Dar es Salaam, Kampala |
| Europe | London, Paris, Berlin, Rome, Madrid, Amsterdam, Stockholm, Oslo, Zurich |
| Americas | New York, Chicago, Denver, Los Angeles, Toronto, São Paulo |
| Asia | Dubai, Riyadh, Kolkata/Mumbai, Shanghai, Tokyo, Singapore, Hong Kong, Seoul, Bangkok, Jakarta, Manila, Ho Chi Minh, Kuala Lumpur |
| Oceania | Sydney, Melbourne, Auckland |

## Work Week Patterns

| Pattern | Days | Countries |
|---------|------|-----------|
| `mon-fri` | Mon–Fri | US, UK, EU, most of Africa |
| `sun-thu` | Sun–Thu | UAE, Saudi, Israel |
| `mon-sat` | Mon–Sat | India, Kenya (some), Japan |
| `sat-thu` | Sat–Thu | Iran |

## Configuration

| Variable | Required | Purpose |
|----------|:--------:|---------|
| `GOOGLE_CALENDAR_TOKEN` | No | Google Calendar sync |
| `MICROSOFT_GRAPH_TOKEN` | No | Outlook/365 sync |
| `CALCOM_API_KEY` | No | Cal.com sync |
| `CALENDLY_TOKEN` | No | Calendly sync |
| `RUST_LOG` | No | Log level |

All integrations are optional. Core scheduling works with zero configuration.

## Use Cases

### Healthcare
```
resource_create (doctors, rooms)
work_week_set (Mon-Sat, 08:00-17:00)
booking_create (patient appointments)
reminder_set (24h + 1h before)
waitlist_join (when doctor is full)
```

### Corporate
```
find_overlap (international team meeting)
sync_google_calendar (pull existing events)
booking_create (with conflict check)
ics_export (send to attendees)
```

### Fitness / Classes
```
group_booking (yoga class, 20 capacity)
booking_link_create (self-service sign-up)
waitlist_join (class full)
shift_create (instructor schedule)
```

### Field Service
```
round_robin_assign (distribute jobs evenly)
buffer_set (30min travel between appointments)
availability_check (find next free slot)
shift_create + shift_update (track field workers)
```

## Error Codes

| Code | Meaning |
|------|---------|
| `CONFLICT` | Resource already booked at that time |
| `TIME_OFF` | Resource is on approved leave |
| `BOOKING_NOT_FOUND` | Booking ID doesn't exist |
| `SHIFT_NOT_FOUND` | Shift ID doesn't exist |
| `ALL_RESOURCES_BUSY` | No available resource in round-robin pool |
| `CAPACITY_FULL` | Group booking at max capacity |
| `NOT_CONFIGURED` | Integration env var not set |

## Documentation

| Document | Description |
|----------|-------------|
| [mcp-server.toml](mcp-server.toml) | ADK-Rust Enterprise registry manifest |
| [Rust Docs](https://docs.rs/mcp-scheduling) | Generated API documentation |

## Contributing

Contributions welcome. Priority areas:
- CalDAV/CardDAV protocol support (Nextcloud, iCloud)
- Recurring event expansion (generate instances from RRULE)
- SMS/email reminder delivery (via mcp-messaging)
- Drag-and-drop rescheduling hints
- Resource utilization analytics

## Contributors

<!-- ALL-CONTRIBUTORS-LIST:START -->
| [<img src="https://github.com/jkmaina.png" width="80px;" alt=""/><br /><sub><b>James Karanja Maina</b></sub>](https://github.com/jkmaina) |
|:---:|
<!-- ALL-CONTRIBUTORS-LIST:END -->

## License

Apache-2.0 — see [LICENSE](LICENSE) for details.

---

Part of the [ADK-Rust Enterprise](https://enterprise.adk-rust.com) MCP server ecosystem.

Built with ❤️ by [Zavora AI](https://zavora.ai)

## Registry Compliance

This server implements the [ADK MCP SDK](https://crates.io/crates/adk-mcp-sdk) contract:

- **HealthCheck** — async health probe for registry monitoring
- **mcp-server.toml** — manifest declaring tools, risk classes, and credentials
- **Structured tracing** — `RUST_LOG` env-filter for observability
