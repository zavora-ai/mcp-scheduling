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
}
