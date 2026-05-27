use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::types::*;

fn uid() -> String { uuid::Uuid::new_v4().to_string()[..8].to_string() }

#[derive(Clone)]
pub struct Store {
    pub resources: Arc<Mutex<HashMap<String, Resource>>>,
    pub bookings: Arc<Mutex<Vec<Booking>>>,
    pub shifts: Arc<Mutex<Vec<Shift>>>,
    pub time_off: Arc<Mutex<Vec<TimeOff>>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            resources: Arc::new(Mutex::new(HashMap::new())),
            bookings: Arc::new(Mutex::new(Vec::new())),
            shifts: Arc::new(Mutex::new(Vec::new())),
            time_off: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn has_conflict(&self, resource_id: &str, start: &str, end: &str, exclude_id: Option<&str>) -> bool {
        self.bookings.lock().unwrap().iter().any(|b| {
            b.resource_id == resource_id && b.status != "cancelled"
            && exclude_id.map_or(true, |eid| b.id != eid)
            && b.start < end.to_string() && b.end > start.to_string()
        })
    }

    pub fn is_on_time_off(&self, resource_id: &str, date: &str) -> bool {
        self.time_off.lock().unwrap().iter().any(|t| {
            t.resource_id == resource_id && t.status == "approved"
            && t.start_date <= date.to_string() && t.end_date >= date.to_string()
        })
    }

    pub fn new_id(prefix: &str) -> String { format!("{}_{}", prefix, uid()) }
}
