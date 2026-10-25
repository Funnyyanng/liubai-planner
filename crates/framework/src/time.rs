use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use chrono_tz::Tz;
use planner_contracts::*;
pub fn now() -> Minute {
    test_now().unwrap_or_else(|| Utc::now().timestamp() / 60)
}
pub fn test_now() -> Option<Minute> {
    std::env::var("LIUBAI_TEST_NOW")
        .ok()
        .and_then(|v| DateTime::parse_from_rfc3339(&v).ok())
        .map(|d| d.timestamp() / 60)
}
pub fn zone(s: &str) -> Result<Tz> {
    s.parse().map_err(|_| AppError::new("timezone", "无效时区"))
}
pub fn local(min: Minute, tz: Tz) -> Result<DateTime<Tz>> {
    Utc.timestamp_opt(min * 60, 0)
        .single()
        .map(|d| d.with_timezone(&tz))
        .ok_or_else(|| AppError::new("time", "无效时间"))
}
pub fn date_key(min: Minute, tz: &str) -> Result<String> {
    Ok(local(min, zone(tz)?)?.format("%Y-%m-%d").to_string())
}
pub fn days(now: Minute, count: u32, tz: &str) -> Result<Vec<(Minute, u32)>> {
    let z = zone(tz)?;
    let day = local(now, z)?.date_naive();
    (0..count)
        .map(|i| {
            let date = day + Duration::days(i.into());
            let t = z
                .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
                .earliest()
                .ok_or_else(|| AppError::new("time", "本地午夜不存在"))?;
            Ok((t.timestamp() / 60, date.weekday().num_days_from_monday()))
        })
        .collect()
}
pub fn day_hour(day: Minute, hour: u32, tz: &str) -> Result<Minute> {
    let z = zone(tz)?;
    let date = local(day, z)?.date_naive();
    let dt = if hour == 24 {
        (date + Duration::days(1)).and_hms_opt(0, 0, 0)
    } else {
        date.and_hms_opt(hour, 0, 0)
    }
    .ok_or_else(|| AppError::new("time", "小时必须为 0 到 24"))?;
    z.from_local_datetime(&dt)
        .earliest()
        .map(|d| d.timestamp() / 60)
        .ok_or_else(|| AppError::new("time", "该本地时间不存在"))
}
