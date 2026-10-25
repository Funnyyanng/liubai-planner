use chrono::{Datelike, Duration, TimeZone};
use planner_contracts::*;
use planner_framework::{time, UnitOfWork};
pub fn list(u: &UnitOfWork) -> Result<Vec<CalendarEvent>> {
    u.repo("calendar")?.all()
}
pub fn save(u: &UnitOfWork, e: &CalendarEvent) -> Result<()> {
    if e.id.is_empty() || e.title.trim().is_empty() || e.end <= e.start {
        return Err(AppError::new("calendar", "日历标题或起止时间无效"));
    }
    if let Some(r) = &e.recurrence {
        if r.interval_weeks == 0
            || r.interval_weeks > 52
            || r.until < e.start
            || r.weekdays.is_empty()
            || r.weekdays.iter().any(|d| *d > 6)
        {
            return Err(AppError::new("recurrence", "重复规则无效"));
        }
    }
    u.repo("calendar")?.put(&e.id, e)?;
    u.emit("calendar.changed", &e.id)
}
pub fn expand(
    events: &[CalendarEvent],
    from: Minute,
    until: Minute,
    tz: &str,
) -> Result<Vec<CalendarEvent>> {
    let z = time::zone(tz)?;
    let mut out = vec![];
    for e in events {
        if let Some(r) = &e.recurrence {
            let origin = time::local(e.start, z)?;
            let origin_date = origin.date_naive();
            let monday =
                origin_date - Duration::days(origin.weekday().num_days_from_monday().into());
            let end_local = time::local(e.end, z)?;
            let wall_duration = end_local.naive_local() - origin.naive_local();
            let mut date = time::local(from, z)?.date_naive().max(origin_date);
            let last = time::local(until.min(r.until), z)?.date_naive();
            while date <= last {
                let weeks = (date - monday).num_days() / 7;
                if weeks % r.interval_weeks as i64 == 0
                    && r.weekdays.contains(&date.weekday().num_days_from_monday())
                    && !r.excluded_dates.contains(&date.to_string())
                {
                    if let Some(start) = z
                        .from_local_datetime(&date.and_time(origin.time()))
                        .earliest()
                    {
                        if let Some(end) = z
                            .from_local_datetime(&(date.and_time(origin.time()) + wall_duration))
                            .earliest()
                        {
                            let mut item = e.clone();
                            item.start = start.timestamp() / 60;
                            item.end = end.timestamp() / 60;
                            item.recurrence = None;
                            if item.end > from && item.start < until && item.start <= r.until {
                                out.push(item)
                            }
                        }
                    }
                }
                date += Duration::days(1);
            }
        } else if e.end > from && e.start < until {
            out.push(e.clone())
        }
    }
    out.sort_by_key(|e| (e.start, e.id.clone()));
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alternating_weeks_and_exception() {
        let from = chrono::DateTime::parse_from_rfc3339("2026-09-07T09:00:00+08:00")
            .unwrap()
            .timestamp()
            / 60;
        let e = CalendarEvent {
            id: "course".into(),
            title: "课程".into(),
            start: from,
            end: from + 60,
            location: "A".into(),
            equipment: vec![],
            recurrence: Some(Recurrence {
                until: from + 40 * 1440,
                interval_weeks: 2,
                weekdays: vec![0],
                excluded_dates: vec!["2026-09-21".into()],
            }),
            source: "manual".into(),
        };
        let out = expand(&[e], from, from + 40 * 1440, "Asia/Shanghai").unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(
            time::date_key(out[1].start, "Asia/Shanghai").unwrap(),
            "2026-10-05"
        );
    }
    #[test]
    fn local_hour_stays_fixed_across_dst() {
        let from = chrono::DateTime::parse_from_rfc3339("2026-10-26T09:00:00-04:00")
            .unwrap()
            .timestamp()
            / 60;
        let e = CalendarEvent {
            id: "dst".into(),
            title: "课程".into(),
            start: from,
            end: from + 60,
            location: "".into(),
            equipment: vec![],
            recurrence: Some(Recurrence {
                until: from + 15 * 1440,
                interval_weeks: 1,
                weekdays: vec![0],
                excluded_dates: vec![],
            }),
            source: "manual".into(),
        };
        let out = expand(&[e], from, from + 15 * 1440, "America/New_York").unwrap();
        assert_eq!(out[1].start - out[0].start, 7 * 1440 + 60);
    }
}
