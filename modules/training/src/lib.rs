use planner_contracts::*;
use planner_framework::{time, UnitOfWork};
pub fn get(u: &UnitOfWork) -> Result<TrainingConfig> {
    Ok(u.repo("training")?.get("config")?.unwrap_or_default())
}
pub fn save(u: &UnitOfWork, c: &TrainingConfig) -> Result<()> {
    if c.sessions == 0 || c.sessions > 7 || c.minutes < 5 || c.minutes > 240 || c.gap_hours > 168 {
        return Err(AppError::new("training", "训练次数、时长或恢复间隔无效"));
    }
    u.repo("training")?.put("config", c)?;
    u.emit("training.changed", c)
}
pub fn contribute(
    c: &TrainingConfig,
    now: Minute,
    tz: &str,
    existing: &[Task],
) -> Result<(Vec<Task>, Vec<Constraint>)> {
    if !c.enabled {
        return Ok((vec![], vec![]));
    }
    let days = time::days(now, 14, tz)?;
    let mut tasks = vec![];
    let mut constraints = vec![];
    let mut groups: std::collections::BTreeMap<String, (Minute, Minute)> =
        std::collections::BTreeMap::new();
    for (day, weekday) in days {
        let start = day - weekday as i64 * 1440;
        let key = time::date_key(start, tz)?;
        groups.entry(key).or_insert((start, start + 7 * 1440));
    }
    let mut all_ids = vec![];
    for (key, (start, end)) in groups {
        let mut ids = vec![];
        for i in 0..c.sessions {
            let id = format!("training:{key}:{i}");
            ids.push(id.clone());
            all_ids.push(id.clone());
            if !existing.iter().any(|t| t.id == id) {
                tasks.push(Task {
                    id: id.clone(),
                    title: format!("{} · 第 {} 次", c.title, i + 1),
                    project: "每周训练".into(),
                    category: "fitness".into(),
                    remaining: c.minutes,
                    estimate: c.minutes,
                    deadline: end,
                    priority: 2,
                    min_chunk: c.minutes,
                    splittable: false,
                    conditions: Conditions {
                        focus: 1,
                        interruptible: false,
                        equipment: vec![],
                        location: None,
                    },
                    dependencies: vec![],
                    completed: false,
                    blocked: false,
                    completion_criteria: "完成训练并记录状态".into(),
                    deferrals: 0,
                    parent_id: None,
                });
            }
            constraints.push(Constraint::Allowed {
                task_id: id,
                start,
                end,
                source: "training".into(),
                reason: "本周训练".into(),
            });
        }
        constraints.push(Constraint::Quota {
            task_ids: ids,
            start,
            end,
            minimum_minutes: c.sessions * c.minutes,
            source: "training".into(),
            reason: format!("{key} 周训练目标"),
        });
    }
    constraints.push(Constraint::MinimumGap {
        task_ids: all_ids,
        minutes: c.gap_hours * 60,
        source: "training".into(),
        reason: "训练间恢复时间".into(),
    });
    Ok((tasks, constraints))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extension_contributes_only_public_contracts() {
        let c = TrainingConfig {
            enabled: true,
            ..Default::default()
        };
        let (ts, cs) = contribute(&c, 29812800, "Asia/Shanghai", &[]).unwrap();
        assert!(ts.len() >= 6);
        assert!(cs
            .iter()
            .any(|c| matches!(c, Constraint::MinimumGap { minutes: 1440, .. })));
        assert!(contribute(&c, 29812800, "Asia/Shanghai", &ts)
            .unwrap()
            .0
            .is_empty());
    }
}
