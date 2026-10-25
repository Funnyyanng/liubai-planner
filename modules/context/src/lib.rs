use planner_contracts::*;
use planner_framework::{time, UnitOfWork};
pub fn list(u: &UnitOfWork) -> Result<Vec<ContextOverride>> {
    u.repo("context")?.all()
}
pub fn save(u: &UnitOfWork, c: &ContextOverride) -> Result<()> {
    u.repo("context")?.put(&c.id, c)?;
    u.emit("context.changed", &c.event_id)
}
pub fn windows(
    base: &[AvailabilityWindow],
    events: &[CalendarEvent],
    overrides: &[ContextOverride],
    tz: &str,
) -> Result<Vec<AvailabilityWindow>> {
    let mut result = vec![];
    for w in base {
        let mut bounds = vec![w.start, w.end];
        for e in events {
            if e.end > w.start && e.start < w.end {
                bounds.push(e.start.max(w.start));
                bounds.push(e.end.min(w.end));
            }
        }
        bounds.sort();
        bounds.dedup();
        for pair in bounds.windows(2) {
            let mut slot = w.clone();
            slot.start = pair[0];
            slot.end = pair[1];
            let mut blocked = false;
            for e in events
                .iter()
                .filter(|e| e.start < slot.end && e.end > slot.start)
            {
                let date = time::date_key(e.start, tz)?;
                let mode = overrides
                    .iter()
                    .find(|o| o.event_id == e.id && o.date.as_ref() == Some(&date))
                    .or_else(|| {
                        overrides
                            .iter()
                            .find(|o| o.event_id == e.id && o.date.is_none())
                    })
                    .map(|o| o.attention.clone())
                    .unwrap_or(Attention::Focused);
                match mode {
                    Attention::Focused => {
                        blocked = true;
                        break;
                    }
                    Attention::Partial => {
                        slot.focus = slot.focus.min(1);
                        slot.interruptible_only = true
                    }
                    Attention::Available => {}
                }
                if slot.location != "不限" && slot.location != e.location && !e.location.is_empty()
                {
                    blocked = true;
                    break;
                }
                slot.location = e.location.clone();
                slot.equipment.retain(|x| e.equipment.contains(x));
            }
            if !blocked && slot.end > slot.start {
                result.push(slot)
            }
        }
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_off_attention_retains_equipment_and_location() {
        let e = CalendarEvent {
            id: "c".into(),
            title: "课".into(),
            start: 100,
            end: 160,
            location: "教室".into(),
            equipment: vec!["电脑".into()],
            recurrence: None,
            source: "manual".into(),
        };
        let base = vec![AvailabilityWindow {
            start: 0,
            end: 200,
            focus: 3,
            interruptible_only: false,
            equipment: vec!["电脑".into(), "实验设备".into()],
            location: "不限".into(),
        }];
        assert_eq!(
            windows(&base, std::slice::from_ref(&e), &[], "UTC")
                .unwrap()
                .len(),
            2
        );
        let ov = ContextOverride {
            id: "o".into(),
            event_id: "c".into(),
            date: None,
            attention: Attention::Partial,
        };
        let out = windows(&base, &[e], &[ov], "UTC").unwrap();
        let w = out.iter().find(|w| w.start == 100).unwrap();
        assert_eq!(w.focus, 1);
        assert!(w.interruptible_only);
        assert_eq!(w.location, "教室");
        assert_eq!(w.equipment, vec!["电脑"]);
    }
}
