use chrono::{DateTime, Local, Utc};

use crate::models::Platform;

/// Formats total playtime in seconds for cards and KPIs.
pub fn format_playtime(seconds: i64) -> String {
    if seconds <= 0 {
        return "Never played".to_string();
    }
    if seconds < 60 {
        return "< 1 min".to_string();
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{minutes} min");
    }
    let hours = seconds as f64 / 3600.0;
    if hours < 10.0 {
        return format!("{hours:.1} hrs");
    }
    format!("{} hrs", hours.round() as i64)
}

/// Formats an exact session duration (`MM:SS` or `Xh YYm ZZs`).
pub fn format_duration_exact(seconds: i64) -> String {
    let safe = seconds.max(0);
    let hrs = safe / 3600;
    let mins = (safe % 3600) / 60;
    let secs = safe % 60;
    if hrs > 0 {
        format!("{hrs}h {mins:02}m {secs:02}s")
    } else {
        format!("{mins:02}:{secs:02}")
    }
}

/// Formats an RFC3339 date relative to the current time.
pub fn format_relative_date(iso: Option<&str>) -> String {
    let Some(raw) = iso.map(str::trim).filter(|s| !s.is_empty()) else {
        return "Never".to_string();
    };
    let Ok(parsed) = DateTime::parse_from_rfc3339(raw) else {
        return "Unknown".to_string();
    };

    let diff_seconds = (Utc::now() - parsed.with_timezone(&Utc)).num_seconds();
    if diff_seconds < 60 {
        return "Just now".to_string();
    }
    let diff_minutes = diff_seconds / 60;
    if diff_minutes < 60 {
        return format!("{diff_minutes}m ago");
    }
    let diff_hours = diff_minutes / 60;
    if diff_hours < 24 {
        return format!("{diff_hours}h ago");
    }
    let diff_days = diff_hours / 24;
    if diff_days == 1 {
        return "Yesterday".to_string();
    }
    if diff_days < 7 {
        return format!("{diff_days}d ago");
    }

    parsed.with_timezone(&Local).format("%Y-%m-%d").to_string()
}

/// Formats an RFC3339 timestamp in local time as `YYYY-MM-DD, HH:MM`.
pub fn format_date_time(iso: &str) -> String {
    let Ok(parsed) = DateTime::parse_from_rfc3339(iso.trim()) else {
        return iso.to_string();
    };
    parsed
        .with_timezone(&Local)
        .format("%Y-%m-%d, %H:%M")
        .to_string()
}

/// Returns the human-readable English platform label.
pub fn platform_label(platform: Platform) -> &'static str {
    match platform {
        Platform::Steam => "Steam",
        Platform::Manual => "Manual",
        Platform::Emulator => "Emulator",
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    #[test]
    fn test_format_playtime() {
        assert_eq!(format_playtime(0), "Never played");
        assert_eq!(format_playtime(-5), "Never played");
        assert_eq!(format_playtime(45), "< 1 min");
        assert_eq!(format_playtime(300), "5 min");
        assert_eq!(format_playtime(5400), "1.5 hrs");
        assert_eq!(format_playtime(45_000), "13 hrs");
    }

    #[test]
    fn test_format_duration_exact() {
        assert_eq!(format_duration_exact(-3), "00:00");
        assert_eq!(format_duration_exact(65), "01:05");
        assert_eq!(format_duration_exact(3661), "1h 01m 01s");
    }

    #[test]
    fn test_format_relative_date() {
        assert_eq!(format_relative_date(None), "Never");
        assert_eq!(format_relative_date(Some("invalid")), "Unknown");

        let just_now = (Utc::now() - Duration::seconds(20)).to_rfc3339();
        assert_eq!(format_relative_date(Some(&just_now)), "Just now");

        let two_hours_ago = (Utc::now() - Duration::hours(2)).to_rfc3339();
        assert_eq!(format_relative_date(Some(&two_hours_ago)), "2h ago");

        let yesterday = (Utc::now() - Duration::hours(30)).to_rfc3339();
        assert_eq!(format_relative_date(Some(&yesterday)), "Yesterday");
    }

    #[test]
    fn test_platform_label_and_date_time() {
        assert_eq!(platform_label(Platform::Steam), "Steam");
        assert_eq!(platform_label(Platform::Manual), "Manual");
        assert_eq!(platform_label(Platform::Emulator), "Emulator");
        assert_eq!(format_date_time("invalid"), "invalid");
        let formatted = format_date_time("2026-11-22T12:34:00Z");
        assert!(formatted.contains("2026-11-22"));
    }
}
