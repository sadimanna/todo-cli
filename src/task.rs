use chrono::{Local, NaiveDateTime, TimeZone};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Todo,
    InProgress,
    Done,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub project_id: Option<i64>,
    pub status: Status,
    pub priority: i64,
    pub deadline: Option<i64>,
    pub reminder: Option<i64>,
}

pub fn parse_datetime_local(input: &str) -> Result<i64, String> {
    let naive = NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M")
        .map_err(|_| format!("Invalid datetime: {} (expected YYYY-MM-DD HH:MM)", input))?;
    let local = Local
        .from_local_datetime(&naive)
        .single()
        .ok_or_else(|| format!("Invalid or ambiguous local time: {}", input))?;
    Ok(local.timestamp())
}

pub fn format_datetime(ts: Option<i64>) -> String {
    match ts {
        Some(value) => Local
            .timestamp_opt(value, 0)
            .single()
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "-".to_string()),
        None => "-".to_string(),
    }
}

pub fn status_label(status: Status) -> &'static str {
    match status {
        Status::Todo => "Todo",
        Status::InProgress => "In Progress",
        Status::Done => "Done",
    }
}

pub fn status_from_db(value: Option<String>) -> Status {
    match value.as_deref() {
        Some("IN_PROGRESS") => Status::InProgress,
        Some("DONE") => Status::Done,
        _ => Status::Todo,
    }
}

pub fn status_to_db(status: Status) -> &'static str {
    match status {
        Status::Todo => "TODO",
        Status::InProgress => "IN_PROGRESS",
        Status::Done => "DONE",
    }
}

pub fn status_column(status: Status) -> usize {
    match status {
        Status::Todo => 0,
        Status::InProgress => 1,
        Status::Done => 2,
    }
}

pub fn status_from_column(column: usize) -> Status {
    match column {
        1 => Status::InProgress,
        2 => Status::Done,
        _ => Status::Todo,
    }
}

pub fn priority_label(priority: i64) -> &'static str {
    match priority {
        4 => "Very High",
        3 => "Medium High",
        2 => "High",
        1 => "Normal",
        0 => "Low",
        _ => "Normal",
    }
}

pub fn normalize_priority(priority: i64) -> i64 {
    priority.clamp(0, 4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_datetime_valid() {
        let ts = parse_datetime_local("2026-03-15 18:00").expect("parse ok");
        assert!(ts > 0);
    }

    #[test]
    fn parse_datetime_invalid() {
        let err = parse_datetime_local("2026-03-15").unwrap_err();
        assert!(err.contains("YYYY-MM-DD HH:MM"));
    }
}
