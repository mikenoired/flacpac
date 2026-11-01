use std::path::PathBuf;

pub(crate) fn reveal_track(path: &PathBuf) {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = std::process::Command::new("open");
        c.arg("-R").arg(path);
        c
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = std::process::Command::new("explorer");
        c.arg("/select,").arg(path);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(path.parent().unwrap_or(path));
        c
    };
    let _ = command.spawn();
}
pub struct SliderValue {
    pub fraction: f32,
    pub maximum: f32,
    pub step: f32,
}

pub(crate) fn unix_now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
pub fn format_added(added: u128, now: u128) -> String {
    if added == 0 {
        return "—".into();
    }
    if added <= now && now - added < 30 * 86_400_000 {
        let seconds = (now - added) / 1000;
        let (count, unit) = if seconds < 60 {
            (seconds, "second")
        } else if seconds < 3600 {
            (seconds / 60, "minute")
        } else if seconds < 86400 {
            (seconds / 3600, "hour")
        } else if seconds < 604800 {
            (seconds / 86400, "day")
        } else {
            (seconds / 604800, "week")
        };
        return format!("{count} {unit}{} ago", if count == 1 { "" } else { "s" });
    }
    // Gregorian civil date from days since the Unix epoch, without locale-dependent parsing.
    let z = (added / 86_400_000).min(i64::MAX as u128 - 719468) as i64 + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    let names = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!("{} {day}, {year}", names[(month - 1) as usize])
}
