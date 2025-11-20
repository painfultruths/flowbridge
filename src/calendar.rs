use chrono::{DateTime, Utc, TimeZone};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct NextMeeting {
    pub summary: String,
    pub start_time: DateTime<Utc>,
}

fn get_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".task-calendar-url")
}

/// Check if calendar URL is configured
pub fn is_authenticated() -> bool {
    get_config_path().exists()
}

/// Save iCal URL to config
pub fn save_ical_url(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_path();
    fs::write(&config_path, url)?;
    Ok(())
}

/// Get saved iCal URL
fn get_ical_url() -> Result<String, Box<dyn std::error::Error>> {
    let config_path = get_config_path();
    if !config_path.exists() {
        return Err("No iCal URL configured. Run: task auth-calendar".into());
    }
    let url = fs::read_to_string(&config_path)?;
    Ok(url.trim().to_string())
}

/// Parse RFC3339 or similar datetime from iCal
fn parse_ical_datetime(dt_str: &str) -> Option<DateTime<Utc>> {
    // iCal format: YYYYMMDDTHHMMSSZ or YYYYMMDDTHHMMSS
    if dt_str.len() >= 15 {
        let year = dt_str[0..4].parse().ok()?;
        let month = dt_str[4..6].parse().ok()?;
        let day = dt_str[6..8].parse().ok()?;
        let hour = dt_str[9..11].parse().ok()?;
        let minute = dt_str[11..13].parse().ok()?;
        let second = dt_str[13..15].parse().ok()?;

        Utc.with_ymd_and_hms(year, month, day, hour, minute, second).single()
    } else {
        None
    }
}

/// Fetch the next upcoming meeting from iCal URL
pub fn get_next_meeting() -> Result<Option<NextMeeting>, Box<dyn std::error::Error>> {
    let url = get_ical_url()?;

    // Fetch iCal data
    let response = reqwest::blocking::get(&url)?;
    let ical_data = response.text()?;

    // Parse iCal
    let reader = ical::IcalParser::new(ical_data.as_bytes());

    let now = Utc::now();
    let mut next_meeting: Option<NextMeeting> = None;

    for calendar_result in reader {
        if let Ok(calendar) = calendar_result {
            for event in calendar.events {
                let mut summary = None;
                let mut start_time = None;

                for property in &event.properties {
                    match property.name.as_str() {
                        "SUMMARY" => {
                            if let Some(value) = &property.value {
                                summary = Some(value.clone());
                            }
                        }
                        "DTSTART" => {
                            if let Some(value) = &property.value {
                                start_time = parse_ical_datetime(value);
                            }
                        }
                        _ => {}
                    }
                }

                if let (Some(summary), Some(start_time)) = (summary, start_time) {
                    // Only consider future events
                    if start_time > now {
                        // Keep the earliest future event
                        if next_meeting.is_none() || start_time < next_meeting.as_ref().unwrap().start_time {
                            next_meeting = Some(NextMeeting {
                                summary,
                                start_time,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(next_meeting)
}

/// Helper to get next meeting synchronously (safe to call from sync context)
pub fn get_next_meeting_sync() -> Option<NextMeeting> {
    if !is_authenticated() {
        return None;
    }

    get_next_meeting().ok().flatten()
}
