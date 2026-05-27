use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub name: String,
    pub resource_type: String, // person, room, equipment, vehicle
    pub capacity: Option<u32>,
    pub timezone: String,
    pub working_hours: Option<WorkingHours>,
    pub tags: Vec<String>,
    pub metadata: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkingHours {
    pub monday: Option<(String, String)>,    // ("09:00", "17:00")
    pub tuesday: Option<(String, String)>,
    pub wednesday: Option<(String, String)>,
    pub thursday: Option<(String, String)>,
    pub friday: Option<(String, String)>,
    pub saturday: Option<(String, String)>,
    pub sunday: Option<(String, String)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Booking {
    pub id: String,
    pub resource_id: String,
    pub title: String,
    pub start: String,  // ISO datetime
    pub end: String,
    pub status: String, // confirmed, tentative, cancelled
    pub booked_by: String,
    pub attendees: Vec<String>,
    pub recurrence: Option<String>, // daily, weekly, biweekly, monthly
    pub notes: Option<String>,
    pub metadata: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shift {
    pub id: String,
    pub resource_id: String,
    pub role: String,
    pub start: String,
    pub end: String,
    pub status: String, // scheduled, confirmed, started, completed, no_show
    pub break_minutes: u32,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeOff {
    pub id: String,
    pub resource_id: String,
    pub start_date: String,
    pub end_date: String,
    pub reason: String, // vacation, sick, personal, public_holiday
    pub status: String, // pending, approved, rejected
}
