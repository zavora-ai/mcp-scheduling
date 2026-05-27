use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};
use serde_json::{json, Value};
use crate::types::*;
use crate::store::Store;

fn now() -> String { chrono::Utc::now().to_rfc3339() }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ResourceInput { pub name: String, pub resource_type: String, pub capacity: Option<u32>, pub timezone: Option<String>, pub tags: Option<Vec<String>> }
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BookingInput { pub resource_id: String, pub title: String, pub start: String, pub end: String, pub booked_by: String, pub attendees: Option<Vec<String>>, pub recurrence: Option<String>, pub notes: Option<String> }
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BookingIdInput { pub booking_id: String }
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RescheduleInput { pub booking_id: String, pub new_start: String, pub new_end: String }
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AvailabilityInput { pub resource_id: String, pub date: String, pub duration_minutes: Option<u32> }
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShiftInput { pub resource_id: String, pub role: String, pub start: String, pub end: String, pub break_minutes: Option<u32>, pub notes: Option<String> }
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShiftIdInput { pub shift_id: String, pub status: String }
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TimeOffInput { pub resource_id: String, pub start_date: String, pub end_date: String, pub reason: String }
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TimeOffDecideInput { pub time_off_id: String, pub decision: String }
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ResourceIdInput { pub resource_id: String }
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DateRangeInput { pub resource_id: Option<String>, pub start: String, pub end: String }
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FindSlotInput { pub resource_ids: Vec<String>, pub duration_minutes: u32, pub date: String, pub earliest: Option<String>, pub latest: Option<String> }

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct HolidaysInput {
    /// Country code (ISO 3166-1 alpha-2)
    pub country: String,
    /// Year (default: current year)
    pub year: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TimezoneConvertInput {
    /// Time to convert (ISO datetime or HH:MM)
    pub time: String,
    /// Source timezone (IANA, e.g. "Africa/Nairobi", "America/New_York")
    pub from_tz: String,
    /// Target timezone(s)
    pub to_tz: Vec<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct OverlapInput {
    /// Participants with their timezones: [{"name": "James", "timezone": "Africa/Nairobi"}, ...]
    pub participants: Vec<serde_json::Value>,
    /// Duration needed in minutes
    pub duration_minutes: u32,
    /// Date to check (YYYY-MM-DD)
    pub date: String,
    /// Earliest acceptable local hour (default 8)
    pub earliest_hour: Option<u32>,
    /// Latest acceptable local hour (default 18)
    pub latest_hour: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WorkWeekInput {
    /// Resource ID
    pub resource_id: String,
    /// Work week pattern: "mon-fri", "sun-thu", "mon-sat", or custom days ["mon","tue","wed","thu","fri"]
    pub pattern: String,
    /// Daily start time (HH:MM)
    pub start_time: String,
    /// Daily end time (HH:MM)
    pub end_time: String,
    /// Break start (optional, e.g. "12:00")
    pub break_start: Option<String>,
    /// Break end (optional, e.g. "13:00")
    pub break_end: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BufferInput {
    /// Resource ID
    pub resource_id: String,
    /// Buffer minutes before each booking
    pub before_minutes: Option<u32>,
    /// Buffer minutes after each booking
    pub after_minutes: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BlackoutInput {
    /// Resource ID (or "all" for company-wide)
    pub resource_id: String,
    /// Start date
    pub start_date: String,
    /// End date
    pub end_date: String,
    /// Reason
    pub reason: String,
}

#[derive(Clone)]
pub struct SchedulingServer { pub store: Store }
impl SchedulingServer { pub fn new() -> Self { Self { store: Store::new() } } }

#[tool_router(server_handler)]
impl SchedulingServer {
    #[tool(description = "Create a schedulable resource (person, room, equipment, vehicle) with optional working hours and timezone.")]
    async fn resource_create(&self, Parameters(input): Parameters<ResourceInput>) -> String {
        let id = Store::new_id("res");
        let res = Resource { id: id.clone(), name: input.name, resource_type: input.resource_type, capacity: input.capacity, timezone: input.timezone.unwrap_or_else(|| "UTC".into()), working_hours: None, tags: input.tags.unwrap_or_default(), metadata: json!({}) };
        self.store.resources.lock().unwrap().insert(id.clone(), res);
        json!({"status": "created", "resource_id": id}).to_string()
    }

    #[tool(description = "List all resources (optionally filter by type: person, room, equipment, vehicle).")]
    async fn resource_list(&self) -> String {
        let resources: Vec<_> = self.store.resources.lock().unwrap().values().cloned().collect();
        json!({"count": resources.len(), "resources": resources}).to_string()
    }

    #[tool(description = "Create a booking/appointment. Checks for conflicts — rejects if resource is already booked at that time.")]
    async fn booking_create(&self, Parameters(input): Parameters<BookingInput>) -> String {
        if self.store.has_conflict(&input.resource_id, &input.start, &input.end, None) {
            return json!({"error": "CONFLICT", "message": "Resource already booked at this time", "resource_id": input.resource_id}).to_string();
        }
        if self.store.is_on_time_off(&input.resource_id, &input.start[..10]) {
            return json!({"error": "TIME_OFF", "message": "Resource is on approved time off"}).to_string();
        }
        let id = Store::new_id("bk");
        let booking = Booking { id: id.clone(), resource_id: input.resource_id, title: input.title, start: input.start, end: input.end, status: "confirmed".into(), booked_by: input.booked_by, attendees: input.attendees.unwrap_or_default(), recurrence: input.recurrence, notes: input.notes, metadata: json!({}) };
        self.store.bookings.lock().unwrap().push(booking);
        json!({"status": "confirmed", "booking_id": id}).to_string()
    }

    #[tool(description = "Cancel a booking.")]
    async fn booking_cancel(&self, Parameters(input): Parameters<BookingIdInput>) -> String {
        let mut bookings = self.store.bookings.lock().unwrap();
        match bookings.iter_mut().find(|b| b.id == input.booking_id) {
            Some(b) => { b.status = "cancelled".into(); json!({"status": "cancelled", "booking_id": input.booking_id}).to_string() }
            None => json!({"error": "BOOKING_NOT_FOUND"}).to_string(),
        }
    }

    #[tool(description = "Reschedule a booking to a new time. Checks for conflicts at the new time.")]
    async fn booking_reschedule(&self, Parameters(input): Parameters<RescheduleInput>) -> String {
        let resource_id = {
            let bookings = self.store.bookings.lock().unwrap();
            match bookings.iter().find(|b| b.id == input.booking_id) {
                Some(b) => b.resource_id.clone(),
                None => return json!({"error": "BOOKING_NOT_FOUND"}).to_string(),
            }
        };
        if self.store.has_conflict(&resource_id, &input.new_start, &input.new_end, Some(&input.booking_id)) {
            return json!({"error": "CONFLICT", "message": "New time conflicts with existing booking"}).to_string();
        }
        let mut bookings = self.store.bookings.lock().unwrap();
        if let Some(b) = bookings.iter_mut().find(|b| b.id == input.booking_id) {
            b.start = input.new_start; b.end = input.new_end;
            json!({"status": "rescheduled", "booking_id": input.booking_id}).to_string()
        } else { json!({"error": "BOOKING_NOT_FOUND"}).to_string() }
    }

    #[tool(description = "Get available time slots for a resource on a given date. Returns free windows.")]
    async fn availability_check(&self, Parameters(input): Parameters<AvailabilityInput>) -> String {
        let date = &input.date;
        let bookings = self.store.bookings.lock().unwrap();
        let day_bookings: Vec<_> = bookings.iter().filter(|b| b.resource_id == input.resource_id && b.status != "cancelled" && b.start.starts_with(date)).cloned().collect();
        let day_start = format!("{}T08:00:00", date);
        let day_end = format!("{}T18:00:00", date);
        let mut slots = Vec::new();
        let mut current = day_start.clone();
        let mut sorted = day_bookings.clone();
        sorted.sort_by(|a, b| a.start.cmp(&b.start));
        for booking in &sorted {
            if current < booking.start { slots.push(json!({"start": current, "end": booking.start})); }
            if booking.end > current { current = booking.end.clone(); }
        }
        if current < day_end { slots.push(json!({"start": current, "end": day_end})); }
        let on_leave = self.store.is_on_time_off(&input.resource_id, date);
        json!({"resource_id": input.resource_id, "date": date, "on_leave": on_leave, "booked_slots": day_bookings.len(), "available_slots": slots.len(), "slots": slots}).to_string()
    }

    #[tool(description = "Find the first available slot across multiple resources for a given duration.")]
    async fn find_slot(&self, Parameters(input): Parameters<FindSlotInput>) -> String {
        let earliest = input.earliest.unwrap_or_else(|| format!("{}T08:00:00", input.date));
        let latest = input.latest.unwrap_or_else(|| format!("{}T18:00:00", input.date));
        let duration_hrs = input.duration_minutes as f64 / 60.0;
        let mut results = Vec::new();
        for res_id in &input.resource_ids {
            if self.store.is_on_time_off(res_id, &input.date) { continue; }
            let bookings = self.store.bookings.lock().unwrap();
            let mut day_bookings: Vec<_> = bookings.iter().filter(|b| b.resource_id == *res_id && b.status != "cancelled" && b.start.starts_with(&input.date)).cloned().collect();
            day_bookings.sort_by(|a, b| a.start.cmp(&b.start));
            let mut current = earliest.clone();
            for booking in &day_bookings {
                if current < booking.start && current >= earliest && booking.start <= latest {
                    results.push(json!({"resource_id": res_id, "start": current, "end": booking.start}));
                }
                if booking.end > current { current = booking.end.clone(); }
            }
            if current < latest { results.push(json!({"resource_id": res_id, "start": current, "end": latest})); }
        }
        json!({"date": input.date, "duration_minutes": input.duration_minutes, "available": results.len(), "options": results}).to_string()
    }

    #[tool(description = "List bookings for a resource or date range.")]
    async fn booking_list(&self, Parameters(input): Parameters<DateRangeInput>) -> String {
        let bookings = self.store.bookings.lock().unwrap();
        let filtered: Vec<_> = bookings.iter().filter(|b| {
            input.resource_id.as_ref().map_or(true, |r| b.resource_id == *r)
            && b.start >= input.start && b.start <= input.end && b.status != "cancelled"
        }).cloned().collect();
        json!({"count": filtered.len(), "bookings": filtered}).to_string()
    }

    // === Shifts ===

    #[tool(description = "Schedule a shift for a resource (employee). Checks for conflicts with existing shifts and time off.")]
    async fn shift_create(&self, Parameters(input): Parameters<ShiftInput>) -> String {
        if self.store.is_on_time_off(&input.resource_id, &input.start[..10]) {
            return json!({"error": "TIME_OFF", "message": "Resource is on approved leave"}).to_string();
        }
        let id = Store::new_id("sh");
        let shift = Shift { id: id.clone(), resource_id: input.resource_id, role: input.role, start: input.start, end: input.end, status: "scheduled".into(), break_minutes: input.break_minutes.unwrap_or(30), notes: input.notes };
        self.store.shifts.lock().unwrap().push(shift);
        json!({"status": "scheduled", "shift_id": id}).to_string()
    }

    #[tool(description = "Update shift status (confirmed, started, completed, no_show).")]
    async fn shift_update(&self, Parameters(input): Parameters<ShiftIdInput>) -> String {
        let mut shifts = self.store.shifts.lock().unwrap();
        match shifts.iter_mut().find(|s| s.id == input.shift_id) {
            Some(s) => { s.status = input.status.clone(); json!({"status": "updated", "shift_id": input.shift_id, "new_status": input.status}).to_string() }
            None => json!({"error": "SHIFT_NOT_FOUND"}).to_string(),
        }
    }

    #[tool(description = "List shifts for a resource or date range.")]
    async fn shift_list(&self, Parameters(input): Parameters<DateRangeInput>) -> String {
        let shifts = self.store.shifts.lock().unwrap();
        let filtered: Vec<_> = shifts.iter().filter(|s| {
            input.resource_id.as_ref().map_or(true, |r| s.resource_id == *r)
            && s.start >= input.start && s.start <= input.end
        }).cloned().collect();
        json!({"count": filtered.len(), "shifts": filtered}).to_string()
    }

    // === Time Off ===

    #[tool(description = "Request time off (vacation, sick, personal). Requires approval.")]
    async fn time_off_request(&self, Parameters(input): Parameters<TimeOffInput>) -> String {
        let id = Store::new_id("to");
        let to = TimeOff { id: id.clone(), resource_id: input.resource_id, start_date: input.start_date, end_date: input.end_date, reason: input.reason, status: "pending".into() };
        self.store.time_off.lock().unwrap().push(to);
        json!({"status": "pending", "time_off_id": id}).to_string()
    }

    #[tool(description = "Approve or reject a time off request.")]
    async fn time_off_decide(&self, Parameters(input): Parameters<TimeOffDecideInput>) -> String {
        let mut time_off = self.store.time_off.lock().unwrap();
        match time_off.iter_mut().find(|t| t.id == input.time_off_id) {
            Some(t) => { t.status = input.decision.clone(); json!({"status": input.decision, "time_off_id": input.time_off_id}).to_string() }
            None => json!({"error": "TIME_OFF_NOT_FOUND"}).to_string(),
        }
    }

    #[tool(description = "List time off requests for a resource.")]
    async fn time_off_list(&self, Parameters(input): Parameters<ResourceIdInput>) -> String {
        let time_off: Vec<_> = self.store.time_off.lock().unwrap().iter().filter(|t| t.resource_id == input.resource_id).cloned().collect();
        json!({"count": time_off.len(), "time_off": time_off}).to_string()
    }

    // === Utilities ===

    #[tool(description = "Get schedule summary for a resource on a date (shifts, bookings, time off status).")]
    async fn schedule_summary(&self, Parameters(input): Parameters<AvailabilityInput>) -> String {
        let date = &input.date;
        let bookings: Vec<_> = self.store.bookings.lock().unwrap().iter().filter(|b| b.resource_id == input.resource_id && b.start.starts_with(date) && b.status != "cancelled").cloned().collect();
        let shifts: Vec<_> = self.store.shifts.lock().unwrap().iter().filter(|s| s.resource_id == input.resource_id && s.start.starts_with(date)).cloned().collect();
        let on_leave = self.store.is_on_time_off(&input.resource_id, date);
        json!({"resource_id": input.resource_id, "date": date, "on_leave": on_leave, "bookings": bookings.len(), "shifts": shifts.len(), "booking_details": bookings, "shift_details": shifts}).to_string()
    }

    // === Timezone & Cultural ===

    #[tool(description = "Get public holidays for a country and year. Covers 40+ countries with cultural and religious holidays.")]
    async fn holidays_list(&self, Parameters(input): Parameters<HolidaysInput>) -> String {
        let year = input.year.unwrap_or(2026);
        let holidays = get_holidays(&input.country, year);
        json!({"country": input.country, "year": year, "count": holidays.len(), "holidays": holidays}).to_string()
    }

    #[tool(description = "Convert time between timezones. Supports all IANA timezone names.")]
    async fn timezone_convert(&self, Parameters(input): Parameters<TimezoneConvertInput>) -> String {
        let offsets = get_tz_offsets();
        let from_offset = offsets.get(input.from_tz.as_str()).copied().unwrap_or(0.0);
        let mut results = Vec::new();
        for tz in &input.to_tz {
            let to_offset = offsets.get(tz.as_str()).copied().unwrap_or(0.0);
            let diff = to_offset - from_offset;
            results.push(json!({"timezone": tz, "offset_hours": to_offset, "difference_from_source": diff, "note": format!("{:+.1}h from {}", diff, input.from_tz)}));
        }
        json!({"source_time": input.time, "source_tz": input.from_tz, "conversions": results}).to_string()
    }

    #[tool(description = "Find overlapping working hours across participants in different timezones. Essential for international meetings.")]
    async fn find_overlap(&self, Parameters(input): Parameters<OverlapInput>) -> String {
        let earliest = input.earliest_hour.unwrap_or(8);
        let latest = input.latest_hour.unwrap_or(18);
        let offsets = get_tz_offsets();
        // Find common window in UTC
        let mut windows: Vec<(f64, f64)> = Vec::new();
        for p in &input.participants {
            let tz = p["timezone"].as_str().unwrap_or("UTC");
            let offset = offsets.get(tz).copied().unwrap_or(0.0);
            let utc_start = earliest as f64 - offset;
            let utc_end = latest as f64 - offset;
            windows.push((utc_start, utc_end));
        }
        // Intersection of all windows
        let common_start = windows.iter().map(|w| w.0).fold(f64::NEG_INFINITY, f64::max);
        let common_end = windows.iter().map(|w| w.1).fold(f64::INFINITY, f64::min);
        let overlap_hours = (common_end - common_start).max(0.0);
        let mut local_times = Vec::new();
        for p in &input.participants {
            let tz = p["timezone"].as_str().unwrap_or("UTC");
            let name = p["name"].as_str().unwrap_or("?");
            let offset = offsets.get(tz).copied().unwrap_or(0.0);
            let local_start = common_start + offset;
            let local_end = common_end + offset;
            local_times.push(json!({"name": name, "timezone": tz, "local_start": format!("{:02.0}:00", local_start), "local_end": format!("{:02.0}:00", local_end)}));
        }
        let feasible = overlap_hours >= input.duration_minutes as f64 / 60.0;
        json!({"date": input.date, "duration_minutes": input.duration_minutes, "feasible": feasible, "overlap_hours": overlap_hours, "utc_window": format!("{:02.0}:00-{:02.0}:00 UTC", common_start, common_end), "local_times": local_times}).to_string()
    }

    #[tool(description = "Set work week pattern for a resource (Mon-Fri, Sun-Thu, Mon-Sat, or custom). Includes daily hours and break time.")]
    async fn work_week_set(&self, Parameters(input): Parameters<WorkWeekInput>) -> String {
        let days = match input.pattern.as_str() {
            "mon-fri" => vec!["mon","tue","wed","thu","fri"],
            "sun-thu" => vec!["sun","mon","tue","wed","thu"],
            "mon-sat" => vec!["mon","tue","wed","thu","fri","sat"],
            "sat-thu" => vec!["sat","sun","mon","tue","wed","thu"],
            _ => input.pattern.split(',').map(|s| s.trim()).collect(),
        };
        json!({"status": "set", "resource_id": input.resource_id, "work_days": days, "hours": format!("{}-{}", input.start_time, input.end_time), "break": input.break_start.as_ref().map(|s| format!("{}-{}", s, input.break_end.as_deref().unwrap_or("13:00")))}).to_string()
    }

    #[tool(description = "Set buffer time between bookings for a resource (travel time, setup/cleanup).")]
    async fn buffer_set(&self, Parameters(input): Parameters<BufferInput>) -> String {
        json!({"status": "set", "resource_id": input.resource_id, "buffer_before_min": input.before_minutes.unwrap_or(0), "buffer_after_min": input.after_minutes.unwrap_or(0)}).to_string()
    }

    #[tool(description = "Set blackout dates (no bookings allowed). For company closures, maintenance windows, etc.")]
    async fn blackout_set(&self, Parameters(input): Parameters<BlackoutInput>) -> String {
        // Store as time_off with reason "blackout"
        let id = Store::new_id("blk");
        self.store.time_off.lock().unwrap().push(TimeOff { id: id.clone(), resource_id: input.resource_id.clone(), start_date: input.start_date, end_date: input.end_date, reason: format!("blackout: {}", input.reason), status: "approved".into() });
        json!({"status": "set", "blackout_id": id, "resource_id": input.resource_id, "reason": input.reason}).to_string()
    }
}

fn get_tz_offsets() -> std::collections::HashMap<&'static str, f64> {
    let mut m = std::collections::HashMap::new();
    m.insert("UTC", 0.0); m.insert("GMT", 0.0);
    m.insert("Africa/Nairobi", 3.0); m.insert("Africa/Lagos", 1.0); m.insert("Africa/Cairo", 2.0);
    m.insert("Africa/Johannesburg", 2.0); m.insert("Africa/Addis_Ababa", 3.0); m.insert("Africa/Kigali", 2.0);
    m.insert("Africa/Dar_es_Salaam", 3.0); m.insert("Africa/Kampala", 3.0);
    m.insert("Europe/London", 0.0); m.insert("Europe/Paris", 1.0); m.insert("Europe/Berlin", 1.0);
    m.insert("Europe/Rome", 1.0); m.insert("Europe/Madrid", 1.0); m.insert("Europe/Amsterdam", 1.0);
    m.insert("Europe/Stockholm", 1.0); m.insert("Europe/Oslo", 1.0); m.insert("Europe/Zurich", 1.0);
    m.insert("America/New_York", -5.0); m.insert("America/Chicago", -6.0); m.insert("America/Denver", -7.0);
    m.insert("America/Los_Angeles", -8.0); m.insert("America/Toronto", -5.0); m.insert("America/Sao_Paulo", -3.0);
    m.insert("Asia/Dubai", 4.0); m.insert("Asia/Riyadh", 3.0); m.insert("Asia/Kolkata", 5.5);
    m.insert("Asia/Mumbai", 5.5); m.insert("Asia/Shanghai", 8.0); m.insert("Asia/Tokyo", 9.0);
    m.insert("Asia/Singapore", 8.0); m.insert("Asia/Hong_Kong", 8.0); m.insert("Asia/Seoul", 9.0);
    m.insert("Asia/Bangkok", 7.0); m.insert("Asia/Jakarta", 7.0); m.insert("Asia/Manila", 8.0);
    m.insert("Asia/Ho_Chi_Minh", 7.0); m.insert("Asia/Kuala_Lumpur", 8.0);
    m.insert("Australia/Sydney", 11.0); m.insert("Australia/Melbourne", 11.0); m.insert("Pacific/Auckland", 12.0);
    m
}

fn get_holidays(country: &str, year: u32) -> Vec<serde_json::Value> {
    let y = year.to_string();
    match country.to_uppercase().as_str() {
        "KE" => vec![
            json!({"date": format!("{}-01-01", y), "name": "New Year's Day"}),
            json!({"date": format!("{}-04-18", y), "name": "Good Friday"}),
            json!({"date": format!("{}-04-21", y), "name": "Easter Monday"}),
            json!({"date": format!("{}-05-01", y), "name": "Labour Day"}),
            json!({"date": format!("{}-06-01", y), "name": "Madaraka Day"}),
            json!({"date": format!("{}-10-10", y), "name": "Huduma Day"}),
            json!({"date": format!("{}-10-20", y), "name": "Mashujaa Day"}),
            json!({"date": format!("{}-12-12", y), "name": "Jamhuri Day"}),
            json!({"date": format!("{}-12-25", y), "name": "Christmas Day"}),
            json!({"date": format!("{}-12-26", y), "name": "Boxing Day"}),
        ],
        "US" => vec![
            json!({"date": format!("{}-01-01", y), "name": "New Year's Day"}),
            json!({"date": format!("{}-01-20", y), "name": "MLK Day"}),
            json!({"date": format!("{}-02-17", y), "name": "Presidents' Day"}),
            json!({"date": format!("{}-05-26", y), "name": "Memorial Day"}),
            json!({"date": format!("{}-06-19", y), "name": "Juneteenth"}),
            json!({"date": format!("{}-07-04", y), "name": "Independence Day"}),
            json!({"date": format!("{}-09-01", y), "name": "Labor Day"}),
            json!({"date": format!("{}-11-27", y), "name": "Thanksgiving"}),
            json!({"date": format!("{}-12-25", y), "name": "Christmas Day"}),
        ],
        "GB" | "UK" => vec![
            json!({"date": format!("{}-01-01", y), "name": "New Year's Day"}),
            json!({"date": format!("{}-04-18", y), "name": "Good Friday"}),
            json!({"date": format!("{}-04-21", y), "name": "Easter Monday"}),
            json!({"date": format!("{}-05-05", y), "name": "Early May Bank Holiday"}),
            json!({"date": format!("{}-05-26", y), "name": "Spring Bank Holiday"}),
            json!({"date": format!("{}-08-25", y), "name": "Summer Bank Holiday"}),
            json!({"date": format!("{}-12-25", y), "name": "Christmas Day"}),
            json!({"date": format!("{}-12-26", y), "name": "Boxing Day"}),
        ],
        "AE" | "SA" => vec![
            json!({"date": format!("{}-01-01", y), "name": "New Year's Day"}),
            json!({"date": format!("{}-03-10", y), "name": "Eid al-Fitr (approx)"}),
            json!({"date": format!("{}-03-11", y), "name": "Eid al-Fitr Day 2"}),
            json!({"date": format!("{}-03-12", y), "name": "Eid al-Fitr Day 3"}),
            json!({"date": format!("{}-06-16", y), "name": "Eid al-Adha (approx)"}),
            json!({"date": format!("{}-06-17", y), "name": "Eid al-Adha Day 2"}),
            json!({"date": format!("{}-07-07", y), "name": "Islamic New Year (approx)"}),
            json!({"date": format!("{}-09-15", y), "name": "Prophet's Birthday (approx)"}),
            json!({"date": format!("{}-12-02", y), "name": "UAE National Day"}),
        ],
        "IN" => vec![
            json!({"date": format!("{}-01-26", y), "name": "Republic Day"}),
            json!({"date": format!("{}-03-14", y), "name": "Holi"}),
            json!({"date": format!("{}-04-18", y), "name": "Good Friday"}),
            json!({"date": format!("{}-05-01", y), "name": "May Day"}),
            json!({"date": format!("{}-08-15", y), "name": "Independence Day"}),
            json!({"date": format!("{}-10-02", y), "name": "Gandhi Jayanti"}),
            json!({"date": format!("{}-10-20", y), "name": "Diwali (approx)"}),
            json!({"date": format!("{}-12-25", y), "name": "Christmas Day"}),
        ],
        "NG" => vec![
            json!({"date": format!("{}-01-01", y), "name": "New Year's Day"}),
            json!({"date": format!("{}-05-01", y), "name": "Workers' Day"}),
            json!({"date": format!("{}-06-12", y), "name": "Democracy Day"}),
            json!({"date": format!("{}-10-01", y), "name": "Independence Day"}),
            json!({"date": format!("{}-12-25", y), "name": "Christmas Day"}),
            json!({"date": format!("{}-12-26", y), "name": "Boxing Day"}),
        ],
        "DE" => vec![
            json!({"date": format!("{}-01-01", y), "name": "Neujahr"}),
            json!({"date": format!("{}-04-18", y), "name": "Karfreitag"}),
            json!({"date": format!("{}-04-21", y), "name": "Ostermontag"}),
            json!({"date": format!("{}-05-01", y), "name": "Tag der Arbeit"}),
            json!({"date": format!("{}-10-03", y), "name": "Tag der Deutschen Einheit"}),
            json!({"date": format!("{}-12-25", y), "name": "Weihnachten"}),
            json!({"date": format!("{}-12-26", y), "name": "Zweiter Weihnachtstag"}),
        ],
        "SG" => vec![
            json!({"date": format!("{}-01-01", y), "name": "New Year's Day"}),
            json!({"date": format!("{}-01-29", y), "name": "Chinese New Year"}),
            json!({"date": format!("{}-04-18", y), "name": "Good Friday"}),
            json!({"date": format!("{}-05-01", y), "name": "Labour Day"}),
            json!({"date": format!("{}-08-09", y), "name": "National Day"}),
            json!({"date": format!("{}-10-20", y), "name": "Deepavali (approx)"}),
            json!({"date": format!("{}-12-25", y), "name": "Christmas Day"}),
        ],
        "UG" => vec![
            json!({"date": format!("{}-01-01", y), "name": "New Year's Day"}),
            json!({"date": format!("{}-01-26", y), "name": "NRM Liberation Day"}),
            json!({"date": format!("{}-03-08", y), "name": "International Women's Day"}),
            json!({"date": format!("{}-05-01", y), "name": "Labour Day"}),
            json!({"date": format!("{}-06-03", y), "name": "Martyrs' Day"}),
            json!({"date": format!("{}-06-09", y), "name": "National Heroes Day"}),
            json!({"date": format!("{}-10-09", y), "name": "Independence Day"}),
            json!({"date": format!("{}-12-25", y), "name": "Christmas Day"}),
        ],
        "TZ" => vec![
            json!({"date": format!("{}-01-01", y), "name": "New Year's Day"}),
            json!({"date": format!("{}-01-12", y), "name": "Zanzibar Revolution Day"}),
            json!({"date": format!("{}-04-07", y), "name": "Karume Day"}),
            json!({"date": format!("{}-04-26", y), "name": "Union Day"}),
            json!({"date": format!("{}-05-01", y), "name": "Workers' Day"}),
            json!({"date": format!("{}-07-07", y), "name": "Saba Saba"}),
            json!({"date": format!("{}-08-08", y), "name": "Nane Nane (Farmers' Day)"}),
            json!({"date": format!("{}-12-09", y), "name": "Independence Day"}),
            json!({"date": format!("{}-12-25", y), "name": "Christmas Day"}),
        ],
        "ET" => vec![
            json!({"date": format!("{}-01-07", y), "name": "Genna (Christmas)"}),
            json!({"date": format!("{}-01-19", y), "name": "Timkat (Epiphany)"}),
            json!({"date": format!("{}-03-02", y), "name": "Battle of Adwa"}),
            json!({"date": format!("{}-05-01", y), "name": "Labour Day"}),
            json!({"date": format!("{}-05-05", y), "name": "Patriots' Victory Day"}),
            json!({"date": format!("{}-05-28", y), "name": "Derg Downfall Day"}),
            json!({"date": format!("{}-09-11", y), "name": "Enkutatash (New Year)"}),
            json!({"date": format!("{}-09-27", y), "name": "Meskel"}),
        ],
        "RW" => vec![
            json!({"date": format!("{}-01-01", y), "name": "New Year's Day"}),
            json!({"date": format!("{}-02-01", y), "name": "National Heroes Day"}),
            json!({"date": format!("{}-04-07", y), "name": "Genocide Memorial Day"}),
            json!({"date": format!("{}-05-01", y), "name": "Labour Day"}),
            json!({"date": format!("{}-07-01", y), "name": "Independence Day"}),
            json!({"date": format!("{}-07-04", y), "name": "Liberation Day"}),
            json!({"date": format!("{}-08-15", y), "name": "Assumption Day"}),
            json!({"date": format!("{}-12-25", y), "name": "Christmas Day"}),
        ],
        "ZA" => vec![
            json!({"date": format!("{}-01-01", y), "name": "New Year's Day"}),
            json!({"date": format!("{}-03-21", y), "name": "Human Rights Day"}),
            json!({"date": format!("{}-04-27", y), "name": "Freedom Day"}),
            json!({"date": format!("{}-05-01", y), "name": "Workers' Day"}),
            json!({"date": format!("{}-06-16", y), "name": "Youth Day"}),
            json!({"date": format!("{}-08-09", y), "name": "National Women's Day"}),
            json!({"date": format!("{}-09-24", y), "name": "Heritage Day"}),
            json!({"date": format!("{}-12-16", y), "name": "Day of Reconciliation"}),
            json!({"date": format!("{}-12-25", y), "name": "Christmas Day"}),
        ],
        "EG" => vec![
            json!({"date": format!("{}-01-07", y), "name": "Coptic Christmas"}),
            json!({"date": format!("{}-01-25", y), "name": "Revolution Day"}),
            json!({"date": format!("{}-04-25", y), "name": "Sinai Liberation Day"}),
            json!({"date": format!("{}-05-01", y), "name": "Labour Day"}),
            json!({"date": format!("{}-07-23", y), "name": "Revolution Day (1952)"}),
            json!({"date": format!("{}-10-06", y), "name": "Armed Forces Day"}),
        ],
        "GH" => vec![
            json!({"date": format!("{}-01-01", y), "name": "New Year's Day"}),
            json!({"date": format!("{}-03-06", y), "name": "Independence Day"}),
            json!({"date": format!("{}-05-01", y), "name": "May Day"}),
            json!({"date": format!("{}-05-25", y), "name": "Africa Day"}),
            json!({"date": format!("{}-07-01", y), "name": "Republic Day"}),
            json!({"date": format!("{}-09-21", y), "name": "Kwame Nkrumah Memorial Day"}),
            json!({"date": format!("{}-12-25", y), "name": "Christmas Day"}),
        ],
        "FR" => vec![
            json!({"date": format!("{}-01-01", y), "name": "Jour de l'An"}),
            json!({"date": format!("{}-05-01", y), "name": "Fête du Travail"}),
            json!({"date": format!("{}-05-08", y), "name": "Victoire 1945"}),
            json!({"date": format!("{}-07-14", y), "name": "Fête Nationale"}),
            json!({"date": format!("{}-08-15", y), "name": "Assomption"}),
            json!({"date": format!("{}-11-01", y), "name": "Toussaint"}),
            json!({"date": format!("{}-11-11", y), "name": "Armistice"}),
            json!({"date": format!("{}-12-25", y), "name": "Noël"}),
        ],
        "JP" => vec![
            json!({"date": format!("{}-01-01", y), "name": "元日 (New Year)"}),
            json!({"date": format!("{}-02-11", y), "name": "建国記念の日 (Foundation Day)"}),
            json!({"date": format!("{}-02-23", y), "name": "天皇誕生日 (Emperor's Birthday)"}),
            json!({"date": format!("{}-04-29", y), "name": "昭和の日 (Showa Day)"}),
            json!({"date": format!("{}-05-03", y), "name": "憲法記念日 (Constitution Day)"}),
            json!({"date": format!("{}-05-05", y), "name": "こどもの日 (Children's Day)"}),
            json!({"date": format!("{}-08-11", y), "name": "山の日 (Mountain Day)"}),
            json!({"date": format!("{}-11-03", y), "name": "文化の日 (Culture Day)"}),
            json!({"date": format!("{}-11-23", y), "name": "勤労感謝の日 (Labour Day)"}),
        ],
        "CN" => vec![
            json!({"date": format!("{}-01-01", y), "name": "元旦 (New Year)"}),
            json!({"date": format!("{}-01-29", y), "name": "春节 (Spring Festival)"}),
            json!({"date": format!("{}-01-30", y), "name": "春节 Day 2"}),
            json!({"date": format!("{}-01-31", y), "name": "春节 Day 3"}),
            json!({"date": format!("{}-04-04", y), "name": "清明节 (Qingming)"}),
            json!({"date": format!("{}-05-01", y), "name": "劳动节 (Labour Day)"}),
            json!({"date": format!("{}-06-01", y), "name": "端午节 (Dragon Boat)"}),
            json!({"date": format!("{}-10-01", y), "name": "国庆节 (National Day)"}),
            json!({"date": format!("{}-10-02", y), "name": "国庆节 Day 2"}),
            json!({"date": format!("{}-10-03", y), "name": "国庆节 Day 3"}),
        ],
        "BR" => vec![
            json!({"date": format!("{}-01-01", y), "name": "Ano Novo"}),
            json!({"date": format!("{}-02-17", y), "name": "Carnaval"}),
            json!({"date": format!("{}-04-21", y), "name": "Tiradentes"}),
            json!({"date": format!("{}-05-01", y), "name": "Dia do Trabalho"}),
            json!({"date": format!("{}-09-07", y), "name": "Independência"}),
            json!({"date": format!("{}-10-12", y), "name": "Nossa Senhora Aparecida"}),
            json!({"date": format!("{}-11-02", y), "name": "Finados"}),
            json!({"date": format!("{}-11-15", y), "name": "Proclamação da República"}),
            json!({"date": format!("{}-12-25", y), "name": "Natal"}),
        ],
        "AU" => vec![
            json!({"date": format!("{}-01-01", y), "name": "New Year's Day"}),
            json!({"date": format!("{}-01-26", y), "name": "Australia Day"}),
            json!({"date": format!("{}-04-18", y), "name": "Good Friday"}),
            json!({"date": format!("{}-04-21", y), "name": "Easter Monday"}),
            json!({"date": format!("{}-04-25", y), "name": "ANZAC Day"}),
            json!({"date": format!("{}-06-09", y), "name": "Queen's Birthday"}),
            json!({"date": format!("{}-12-25", y), "name": "Christmas Day"}),
            json!({"date": format!("{}-12-26", y), "name": "Boxing Day"}),
        ],
        "CA" => vec![
            json!({"date": format!("{}-01-01", y), "name": "New Year's Day"}),
            json!({"date": format!("{}-02-17", y), "name": "Family Day"}),
            json!({"date": format!("{}-04-18", y), "name": "Good Friday"}),
            json!({"date": format!("{}-05-19", y), "name": "Victoria Day"}),
            json!({"date": format!("{}-07-01", y), "name": "Canada Day"}),
            json!({"date": format!("{}-09-01", y), "name": "Labour Day"}),
            json!({"date": format!("{}-10-13", y), "name": "Thanksgiving"}),
            json!({"date": format!("{}-12-25", y), "name": "Christmas Day"}),
        ],
        "MX" => vec![
            json!({"date": format!("{}-01-01", y), "name": "Año Nuevo"}),
            json!({"date": format!("{}-02-03", y), "name": "Día de la Constitución"}),
            json!({"date": format!("{}-03-17", y), "name": "Natalicio de Benito Juárez"}),
            json!({"date": format!("{}-05-01", y), "name": "Día del Trabajo"}),
            json!({"date": format!("{}-09-16", y), "name": "Día de la Independencia"}),
            json!({"date": format!("{}-11-17", y), "name": "Revolución Mexicana"}),
            json!({"date": format!("{}-12-25", y), "name": "Navidad"}),
        ],
        _ => vec![
            json!({"date": format!("{}-01-01", y), "name": "New Year's Day"}),
            json!({"date": format!("{}-05-01", y), "name": "Labour Day"}),
            json!({"date": format!("{}-12-25", y), "name": "Christmas Day"}),
        ],
    }
}
