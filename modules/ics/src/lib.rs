use chrono::{NaiveDate, NaiveDateTime, TimeZone, Utc};
use planner_contracts::*;
use planner_framework::time;
fn parse_time(key: &str, value: &str, tz: &str) -> Result<Minute> {
    let z = key
        .split(';')
        .find_map(|p| p.strip_prefix("TZID="))
        .unwrap_or(tz);
    let zone = time::zone(z)?;
    if value.ends_with('Z') {
        let d = NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%SZ")
            .map_err(|_| AppError::new("ics_time", "不支持的 UTC 时间格式"))?;
        return Ok(d.and_utc().timestamp() / 60);
    }
    let d = if value.len() == 8 {
        NaiveDate::parse_from_str(value, "%Y%m%d")
            .ok()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
    } else {
        NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S").ok()
    }
    .ok_or_else(|| AppError::new("ics_time", "不支持的日历时间格式"))?;
    zone.from_local_datetime(&d)
        .earliest()
        .map(|d| d.timestamp() / 60)
        .ok_or_else(|| AppError::new("ics_time", "日历时间在指定时区不存在"))
}
fn unescape(s: &str) -> String {
    s.replace("\\n", "\n")
        .replace("\\N", "\n")
        .replace("\\,", ",")
        .replace("\\;", ";")
        .replace("\\\\", "\\")
}
pub fn parse(input: &str, tz: &str) -> Result<Vec<CalendarEvent>> {
    if input.len() > 5_000_000 {
        return Err(AppError::new("ics_size", "日历文件超过 5 MB"));
    }
    if !input.contains("BEGIN:VCALENDAR") {
        return Err(AppError::new("ics", "不是有效的 ICS 日历文件"));
    }
    let normalized = input.replace("\r\n", "\n");
    let mut lines: Vec<String> = vec![];
    for line in normalized.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(last) = lines.last_mut() {
                last.push_str(&line[1..]);
            }
        } else {
            lines.push(line.into());
        }
    }
    let mut fields: Vec<(String, String)> = vec![];
    let mut inside = false;
    let mut out = vec![];
    for line in lines {
        if line == "BEGIN:VEVENT" {
            inside = true;
            fields.clear();
            continue;
        }
        if line == "END:VEVENT" {
            let field = |name: &str| {
                fields
                    .iter()
                    .find(|(k, _)| k.split(';').next() == Some(name))
            };
            if field("RECURRENCE-ID").is_some() {
                return Err(AppError::new(
                    "ics_recurrence",
                    "文件含修改后的重复实例，请先导出为独立事件后导入",
                ));
            }
            let (k, v) =
                field("DTSTART").ok_or_else(|| AppError::new("ics", "事件缺少 DTSTART"))?;
            let start = parse_time(k, v, tz)?;
            let end = if let Some((k, v)) = field("DTEND") {
                parse_time(k, v, tz)?
            } else if v.len() == 8 {
                start + 1440
            } else {
                return Err(AppError::new("ics", "事件缺少结束时间"));
            };
            let id = field("UID")
                .map(|(_, v)| v.clone())
                .ok_or_else(|| AppError::new("ics", "事件缺少 UID，无法保证重复导入幂等"))?;
            let mut recurrence = None;
            if let Some((_, rr)) = field("RRULE") {
                let pairs: std::collections::BTreeMap<_, _> =
                    rr.split(';').filter_map(|p| p.split_once('=')).collect();
                if pairs.get("FREQ") != Some(&"WEEKLY")
                    || pairs
                        .keys()
                        .any(|k| !["FREQ", "INTERVAL", "BYDAY", "UNTIL", "WKST"].contains(k))
                {
                    return Err(AppError::new(
                        "ics_recurrence",
                        "第一版支持带 UNTIL 的 WEEKLY 重复；COUNT、月度等规则请展开后导入",
                    ));
                }
                let until = parse_time(
                    "DTSTART",
                    pairs.get("UNTIL").ok_or_else(|| {
                        AppError::new("ics_recurrence", "重复课程需要结束日期 UNTIL")
                    })?,
                    tz,
                )?;
                let weekdays = if let Some(days) = pairs.get("BYDAY") {
                    days.split(',')
                        .map(|d| {
                            ["MO", "TU", "WE", "TH", "FR", "SA", "SU"]
                                .iter()
                                .position(|x| *x == d)
                                .map(|n| n as u32)
                                .ok_or_else(|| AppError::new("ics_recurrence", "不支持的 BYDAY"))
                        })
                        .collect::<Result<Vec<_>>>()?
                } else {
                    use chrono::Datelike;
                    vec![time::local(start, time::zone(tz)?)?
                        .weekday()
                        .num_days_from_monday()]
                };
                let interval = pairs
                    .get("INTERVAL")
                    .unwrap_or(&"1")
                    .parse()
                    .map_err(|_| AppError::new("ics_recurrence", "无效重复间隔"))?;
                let mut excluded_dates = vec![];
                for (k, v) in &fields {
                    if k.starts_with("EXDATE") {
                        for date in v.split(',') {
                            excluded_dates.push(time::date_key(parse_time(k, date, tz)?, tz)?);
                        }
                    }
                }
                recurrence = Some(Recurrence {
                    until,
                    interval_weeks: interval,
                    weekdays,
                    excluded_dates,
                });
            }
            out.push(CalendarEvent {
                id: format!("ics:{id}"),
                title: field("SUMMARY")
                    .map(|(_, v)| unescape(v))
                    .unwrap_or("未命名日程".into()),
                start,
                end,
                location: field("LOCATION")
                    .map(|(_, v)| unescape(v))
                    .unwrap_or_default(),
                equipment: vec![],
                recurrence,
                source: "ics".into(),
            });
            inside = false;
            continue;
        }
        if inside {
            if let Some((k, v)) = line.split_once(':') {
                fields.push((k.into(), v.into()));
            }
        }
    }
    if inside {
        return Err(AppError::new("ics", "日历文件不完整"));
    }
    Ok(out)
}
pub fn export(blocks: &[TimeBlock], tasks: &[Task]) -> String {
    let mut s = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Liubai//Planner//ZH\r\n".to_string();
    let stamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    for b in blocks {
        let title = tasks
            .iter()
            .find(|t| t.id == b.task_id)
            .map(|t| t.title.as_str())
            .unwrap_or("任务")
            .replace('\\', "\\\\")
            .replace('\n', "\\n")
            .replace(',', "\\,")
            .replace(';', "\\;");
        let dt = |m| {
            Utc.timestamp_opt(m * 60, 0)
                .single()
                .unwrap()
                .format("%Y%m%dT%H%M%SZ")
                .to_string()
        };
        s.push_str(&format!("BEGIN:VEVENT\r\nUID:{}@liubai\r\nDTSTAMP:{}\r\nDTSTART:{}\r\nDTEND:{}\r\nSUMMARY:{}\r\nEND:VEVENT\r\n",b.id,stamp,dt(b.start),dt(b.end),title));
    }
    s.push_str("END:VCALENDAR\r\n");
    s
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn folded_unicode_and_timezone() {
        let s="BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:abc\r\nDTSTART;TZID=Asia/Shanghai:20260907T090000\r\nDTEND;TZID=Asia/Shanghai:20260907T100000\r\nSUMMARY:高等\r\n 数学\r\nEND:VEVENT\r\nEND:VCALENDAR";
        let e = parse(s, "Asia/Shanghai").unwrap();
        assert_eq!(e[0].title, "高等数学");
        assert_eq!(e[0].end - e[0].start, 60);
        assert_eq!(e[0].id, "ics:abc");
    }
    #[test]
    fn rejects_unsupported_instead_of_losing_events() {
        assert!(parse("BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:x\nDTSTART:20260907T090000Z\nDTEND:20260907T100000Z\nRRULE:FREQ=MONTHLY\nEND:VEVENT\nEND:VCALENDAR","UTC").is_err());
    }
}
