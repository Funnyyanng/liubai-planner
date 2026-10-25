use planner_contracts::*;
use planner_framework::{time, UnitOfWork};
pub fn get(u: &UnitOfWork) -> Result<LifeRules> {
    Ok(u.repo("life")?.get("rules")?.unwrap_or_default())
}
pub fn save(u: &UnitOfWork, r: &LifeRules) -> Result<()> {
    time::zone(&r.timezone)?;
    if r.day_start >= r.day_end
        || r.day_end > 24
        || r.buffer_percent > 50
        || r.protected.iter().any(|p| {
            p.start_hour >= p.end_hour || p.end_hour > 24 || p.weekdays.iter().any(|d| *d > 6)
        })
    {
        return Err(AppError::new("rules", "作息时间或缓冲比例无效"));
    }
    u.repo("life")?.put("rules", r)?;
    u.emit("rules.changed", &"rules")
}
pub fn availability(r: &LifeRules, now: Minute, days: u32) -> Result<Vec<AvailabilityWindow>> {
    let mut windows = vec![];
    for (day, weekday) in time::days(now, days, &r.timezone)? {
        let start = time::day_hour(day, r.day_start, &r.timezone)?.max(now);
        let end = time::day_hour(day, r.day_end, &r.timezone)?;
        let mut spans = vec![(start, end)];
        for p in r.protected.iter().filter(|p| p.weekdays.contains(&weekday)) {
            let a = time::day_hour(day, p.start_hour, &r.timezone)?;
            let b = time::day_hour(day, p.end_hour, &r.timezone)?;
            spans = spans
                .into_iter()
                .flat_map(|(s, e)| {
                    if b <= s || a >= e {
                        vec![(s, e)]
                    } else {
                        vec![(s, a.max(s)), (b.min(e), e)]
                    }
                })
                .filter(|(s, e)| e > s)
                .collect();
        }
        for (s, e) in spans {
            if e > s {
                windows.push(AvailabilityWindow {
                    start: s,
                    end: e,
                    focus: 3,
                    interruptible_only: false,
                    equipment: r.equipment.clone(),
                    location: r.location.clone(),
                })
            }
        }
    }
    Ok(windows)
}
/// Reserve flexibility after calendar and attention restrictions are resolved.
pub fn reserve_buffer(
    r: &LifeRules,
    windows: &[AvailabilityWindow],
) -> Result<Vec<AvailabilityWindow>> {
    let mut groups: std::collections::BTreeMap<String, Vec<AvailabilityWindow>> =
        std::collections::BTreeMap::new();
    for w in windows {
        groups
            .entry(time::date_key(w.start, &r.timezone)?)
            .or_default()
            .push(w.clone());
    }
    let mut out = vec![];
    for spans in groups.values_mut() {
        spans.sort_by_key(|w| w.start);
        let total: i64 = spans.iter().map(|w| w.end - w.start).sum();
        let mut reserve = (total * r.buffer_percent as i64 + 99) / 100;
        for w in spans.iter_mut().rev() {
            let take = reserve.min(w.end - w.start);
            w.end -= take;
            reserve -= take;
        }
        out.extend(spans.iter().filter(|w| w.end > w.start).cloned());
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protected_periods_and_buffer_not_double_counted() {
        let r = LifeRules {
            timezone: "UTC".into(),
            day_start: 8,
            day_end: 18,
            buffer_percent: 10,
            ..Default::default()
        };
        let w = reserve_buffer(&r, &availability(&r, 0, 1).unwrap()).unwrap();
        assert!(w.iter().all(|w| !(w.start < 13 * 60 && w.end > 12 * 60)));
        let total: i64 = w.iter().map(|w| w.end - w.start).sum();
        assert_eq!(total, 9 * 60 * 9 / 10);
    }
}
