use planner_contracts::*;
use std::collections::{BTreeMap, BTreeSet};
fn ceil5(n: i64) -> i64 {
    (n + 4).div_euclid(5) * 5
}
fn total(t: &Task) -> i64 {
    ceil5(t.remaining as i64)
}
fn assigned(id: &str, blocks: &[TimeBlock]) -> i64 {
    blocks
        .iter()
        .filter(|b| b.task_id == id)
        .map(|b| b.end - b.start)
        .sum()
}
fn matches(c: &Conditions, w: &AvailabilityWindow) -> bool {
    c.focus <= w.focus
        && (!w.interruptible_only || c.interruptible)
        && c.equipment.iter().all(|x| w.equipment.contains(x))
        && c.location
            .as_ref()
            .is_none_or(|x| x == &w.location || x == "不限")
}
fn deps<'a>(s: &'a PlanningSnapshot, t: &'a Task) -> Vec<&'a str> {
    let mut out: Vec<_> = t.dependencies.iter().map(String::as_str).collect();
    for c in &s.constraints {
        if let Constraint::Dependency { before, after, .. } = c {
            if after == &t.id {
                out.push(before)
            }
        }
    }
    out
}
fn ready_at(s: &PlanningSnapshot, t: &Task, blocks: &[TimeBlock]) -> Option<Minute> {
    let mut ready = s.now;
    for dep in deps(s, t) {
        if s.completed_ids.iter().any(|id| id == dep) {
            continue;
        }
        let d = s.demands.iter().find(|d| d.task.id == dep)?;
        if d.task.blocked || assigned(dep, blocks) < total(&d.task) {
            return None;
        }
        ready = ready.max(
            blocks
                .iter()
                .filter(|b| b.task_id == dep)
                .map(|b| b.end)
                .max()?,
        );
    }
    Some(ready)
}
fn min_chunk(s: &PlanningSnapshot, t: &Task) -> i64 {
    s.constraints
        .iter()
        .filter_map(|c| {
            if let Constraint::MinimumChunk {
                task_id, minutes, ..
            } = c
            {
                (task_id == &t.id).then_some(*minutes as i64)
            } else {
                None
            }
        })
        .fold(t.min_chunk as i64, i64::max)
}
fn allowed(
    s: &PlanningSnapshot,
    t: &Task,
    start: Minute,
    end: Minute,
    blocks: &[TimeBlock],
) -> bool {
    if end <= start || start < s.now || end > s.until || end > t.deadline {
        return false;
    }
    if !s
        .windows
        .iter()
        .any(|w| w.start <= start && w.end >= end && matches(&t.conditions, w))
    {
        return false;
    }
    if blocks.iter().any(|b| b.start < end && b.end > start) {
        return false;
    }
    for c in &s.constraints {
        match c {
            Constraint::Fixed {
                start: a, end: b, ..
            } => {
                if *a < end && *b > start {
                    return false;
                }
            }
            Constraint::Allowed {
                task_id,
                start: a,
                end: b,
                ..
            } => {
                if task_id == &t.id && (start < *a || end > *b) {
                    return false;
                }
            }
            Constraint::Conditions {
                task_id,
                conditions,
                ..
            } => {
                if task_id == &t.id
                    && !s
                        .windows
                        .iter()
                        .any(|w| w.start <= start && w.end >= end && matches(conditions, w))
                {
                    return false;
                }
            }
            Constraint::MinimumGap {
                task_ids, minutes, ..
            } if task_ids.contains(&t.id) => {
                for b in blocks
                    .iter()
                    .filter(|b| b.task_id != t.id && task_ids.contains(&b.task_id))
                {
                    if !(end + *minutes as i64 <= b.start || start >= b.end + *minutes as i64) {
                        return false;
                    }
                }
            }
            _ => {}
        }
    }
    true
}
fn check_snapshot(s: &PlanningSnapshot) -> Result<()> {
    if s.until <= s.now || s.demands.len() > 10000 {
        return Err(AppError::new("snapshot", "规划时间范围或任务数量无效"));
    }
    let mut map = BTreeMap::new();
    for d in &s.demands {
        let t = &d.task;
        if t.min_chunk == 0
            || t.remaining > 100000
            || t.priority > 3
            || t.conditions.focus > 3
            || map.insert(t.id.clone(), t).is_some()
        {
            return Err(AppError::new("invalid_demand", "任务标识或约束无效"));
        }
    }
    for c in &s.constraints {
        let refs: Vec<&String> = match c {
            Constraint::Allowed { task_id, .. }
            | Constraint::MinimumChunk { task_id, .. }
            | Constraint::Conditions { task_id, .. } => vec![task_id],
            Constraint::MinimumGap { task_ids, .. } | Constraint::Quota { task_ids, .. } => {
                task_ids.iter().collect()
            }
            Constraint::Dependency { before, after, .. } => vec![before, after],
            _ => vec![],
        };
        for id in refs {
            if !map.contains_key(id) && !s.completed_ids.contains(id) {
                return Err(AppError::new(
                    "unknown_constraint_task",
                    format!("约束引用了不存在的任务 {id}"),
                ));
            }
        }
    }
    for c in &s.constraints {
        match c {
            Constraint::Fixed { start, end, .. }
            | Constraint::Allowed { start, end, .. }
            | Constraint::Quota { start, end, .. } => {
                if end <= start {
                    return Err(AppError::new("constraint", "约束时间范围无效"));
                }
            }
            Constraint::MinimumChunk { minutes, .. } if *minutes == 0 => {
                return Err(AppError::new("constraint", "连续时长不能为零"));
            }
            _ => {}
        }
    }
    fn visit(
        id: &str,
        s: &PlanningSnapshot,
        map: &BTreeMap<String, &Task>,
        vis: &mut BTreeSet<String>,
        stack: &mut BTreeSet<String>,
    ) -> Result<()> {
        if s.completed_ids.iter().any(|x| x == id) || vis.contains(id) {
            return Ok(());
        }
        if !stack.insert(id.into()) {
            return Err(AppError::new("dependency_cycle", "前置依赖成环"));
        }
        let t = map
            .get(id)
            .ok_or_else(|| AppError::new("missing_dependency", id))?;
        for dep in deps(s, t) {
            visit(dep, s, map, vis, stack)?
        }
        stack.remove(id);
        vis.insert(id.into());
        Ok(())
    }
    let mut vis = BTreeSet::new();
    for id in map.keys() {
        visit(id, s, &map, &mut vis, &mut BTreeSet::new())?
    }
    Ok(())
}
fn solve(s: &PlanningSnapshot, preserve: bool) -> Result<PlanProposal> {
    let started = std::time::Instant::now();
    let mut timed_out = false;
    let mut blocks = vec![];
    let mut old = s.current.clone();
    old.sort_by_key(|b| (b.start, b.id.clone()));
    for b in old.iter().filter(|b| b.locked || b.active) {
        let t = s
            .demands
            .iter()
            .find(|d| d.task.id == b.task_id)
            .ok_or_else(|| AppError::new("protected_conflict", "锁定时间块的任务已不可用"))?;
        if t.task.blocked
            || !allowed(s, &t.task, b.start, b.end, &blocks)
            || assigned(&b.task_id, &blocks) + (b.end - b.start) > total(&t.task)
        {
            return Err(AppError::new(
                "protected_conflict",
                "锁定或正在执行的安排与最新约束冲突，请先处理该安排",
            ));
        }
        blocks.push(b.clone());
    }
    if preserve {
        for b in old.iter().filter(|b| !b.locked && !b.active) {
            if let Some(d) = s.demands.iter().find(|d| d.task.id == b.task_id) {
                if !d.task.blocked
                    && allowed(s, &d.task, b.start, b.end, &blocks)
                    && assigned(&b.task_id, &blocks) + b.end - b.start <= total(&d.task)
                {
                    blocks.push(b.clone());
                }
            }
        }
        loop {
            let invalid: Vec<_> = blocks
                .iter()
                .filter(|b| !b.locked && !b.active)
                .filter(|b| {
                    let t = &s
                        .demands
                        .iter()
                        .find(|d| d.task.id == b.task_id)
                        .unwrap()
                        .task;
                    ready_at(s, t, &blocks).is_none_or(|ready| b.start < ready)
                })
                .map(|b| b.id.clone())
                .collect();
            if invalid.is_empty() {
                break;
            }
            blocks.retain(|b| !invalid.contains(&b.id));
        }
    }
    // A preserved fragment may leave less than the minimum executable chunk.
    for d in &s.demands {
        let remaining = total(&d.task) - assigned(&d.task.id, &blocks);
        if remaining > 0 && remaining < ceil5(min_chunk(s, &d.task)).min(total(&d.task)) {
            if let Some(i) = blocks
                .iter()
                .rposition(|b| b.task_id == d.task.id && !b.locked && !b.active)
            {
                blocks.remove(i);
            }
        }
    }
    let mut ordered: Vec<_> = s.demands.iter().collect();
    ordered.sort_by_key(|d| (d.task.priority, d.task.deadline, d.task.id.clone()));
    let mut visited = BTreeSet::new();
    'planning: loop {
        let mut progressed = false;
        for d in &ordered {
            let t = &d.task;
            if visited.contains(&t.id) || t.blocked {
                continue;
            }
            let Some(ready) = ready_at(s, t, &blocks) else {
                continue;
            };
            visited.insert(t.id.clone());
            progressed = true;
            let mut remaining = total(t) - assigned(&t.id, &blocks);
            if remaining <= 0 {
                continue;
            }
            let earliest = ceil5(ready.max(d.earliest).max(s.now));
            for w in &s.windows {
                if remaining <= 0 {
                    break;
                }
                let mut start = ceil5(w.start.max(earliest));
                let limit = w.end.min(t.deadline).min(s.until);
                while start < limit && remaining > 0 {
                    if started.elapsed().as_millis() > 1200 {
                        timed_out = true;
                        break 'planning;
                    }
                    if !allowed(s, t, start, (start + 5).min(limit), &blocks) {
                        start += 5;
                        continue;
                    }
                    let mut end = start;
                    while end + 5 <= limit
                        && end - start < remaining
                        && allowed(s, t, end, end + 5, &blocks)
                    {
                        end += 5;
                    }
                    let length = end - start;
                    let need = if t.splittable {
                        ceil5(min_chunk(s, t)).min(total(t))
                    } else {
                        remaining
                    };
                    if length >= need && length > 0 {
                        let duration = length.min(remaining);
                        blocks.push(TimeBlock {
                            id: format!("{}@{}", t.id, start),
                            task_id: t.id.clone(),
                            start,
                            end: start + duration,
                            locked: false,
                            active: false,
                        });
                        remaining -= duration;
                        start += duration;
                    } else {
                        start = if end > start { end + 5 } else { start + 5 };
                    }
                }
            }
        }
        if !progressed {
            break;
        }
    }
    blocks.sort_by_key(|b| (b.start, b.task_id.clone()));
    let mut unscheduled = vec![];
    let mut risks = vec![];
    if timed_out {
        risks.push("已达到计算时限，保留已校验的安排，其余工作仍待安排".into());
    }
    for d in &s.demands {
        let t = &d.task;
        let missing = (total(t) - assigned(&t.id, &blocks)).max(0) as u32;
        if missing > 0 {
            let reason = if t.blocked {
                "任务被阻塞，需解除条件"
            } else if ready_at(s, t, &blocks).is_none() {
                "前置任务尚无完整可行安排"
            } else if t.deadline <= s.now {
                "任务已超过截止时间"
            } else {
                "当前未找到满足时间、情境和生活保障的安排"
            };
            unscheduled.push(Unscheduled {
                task_id: t.id.clone(),
                minutes: missing.min(t.remaining),
                reason: reason.into(),
            });
        }
        if t.deferrals >= 2 {
            risks.push(format!(
                "{} 已连续延期 {} 次，建议复查估时和任务拆解",
                t.title, t.deferrals
            ));
        }
    }
    for c in &s.constraints {
        if let Constraint::Quota {
            task_ids,
            start,
            end,
            minimum_minutes,
            reason,
            ..
        } = c
        {
            let got: i64 = blocks
                .iter()
                .filter(|b| task_ids.contains(&b.task_id))
                .map(|b| (b.end.min(*end) - b.start.max(*start)).max(0))
                .sum();
            if got < *minimum_minutes as i64 {
                risks.push(format!(
                    "{}：仍缺 {} 分钟",
                    reason,
                    *minimum_minutes as i64 - got
                ));
            }
        }
    }
    let mut changes = vec![];
    let ids: BTreeSet<_> = s
        .current
        .iter()
        .chain(blocks.iter())
        .map(|b| b.task_id.clone())
        .collect();
    for id in ids {
        let before: Vec<_> = s
            .current
            .iter()
            .filter(|b| b.task_id == id)
            .cloned()
            .collect();
        let after: Vec<_> = blocks.iter().filter(|b| b.task_id == id).cloned().collect();
        if before != after {
            changes.push(PlanChange {
                task_id: id,
                before,
                after,
                reason: "根据剩余工作量、依赖和可用时间重新安排".into(),
            })
        }
    }
    Ok(PlanProposal {
        input_version: s.version,
        blocks,
        unscheduled,
        risks,
        changes,
        requires_approval: false,
    })
}
fn cost(s: &PlanningSnapshot, p: &PlanProposal) -> u64 {
    p.unscheduled
        .iter()
        .map(|u| {
            u.minutes as u64
                * (4 - s
                    .demands
                    .iter()
                    .find(|d| d.task.id == u.task_id)
                    .map(|d| d.task.priority)
                    .unwrap_or(3)) as u64
        })
        .sum()
}
pub fn propose(s: &PlanningSnapshot) -> Result<PlanProposal> {
    check_snapshot(s)?;
    let stable = solve(s, true)?;
    let p = if validate(s, &stable).is_err() || !stable.unscheduled.is_empty() {
        let fresh = solve(s, false)?;
        if validate(s, &stable).is_err() || cost(s, &fresh) < cost(s, &stable) {
            fresh
        } else {
            stable
        }
    } else {
        stable
    };
    validate(s, &p)?;
    Ok(p)
}
pub fn validate(s: &PlanningSnapshot, p: &PlanProposal) -> Result<()> {
    check_snapshot(s)?;
    if p.input_version != s.version {
        return Err(AppError::new("stale_version", "计划输入版本已过期"));
    }
    let mut placed = vec![];
    let mut ids = BTreeSet::new();
    for b in &p.blocks {
        if !ids.insert(&b.id) {
            return Err(AppError::new("duplicate_block", "重复时间块"));
        }
        let d = s
            .demands
            .iter()
            .find(|d| d.task.id == b.task_id)
            .ok_or_else(|| AppError::new("unknown_task", "时间块关联未知任务"))?;
        if d.task.blocked || b.start < d.earliest || !allowed(s, &d.task, b.start, b.end, &placed) {
            return Err(AppError::new(
                "invalid_plan",
                format!("{} 的时间或执行条件冲突", d.task.title),
            ));
        }
        if ready_at(s, &d.task, &p.blocks).is_none_or(|r| b.start < r) {
            return Err(AppError::new(
                "dependency_violation",
                "前置任务没有完整安排在后继之前",
            ));
        }
        placed.push(b.clone());
    }
    for old in s.current.iter().filter(|b| b.locked || b.active) {
        if !p.blocks.contains(old) {
            return Err(AppError::new(
                "protected_conflict",
                "不能自动移动锁定或执行中的安排",
            ));
        }
    }
    for d in &s.demands {
        let got = assigned(&d.task.id, &p.blocks);
        if got > total(&d.task) {
            return Err(AppError::new("overallocated", "重复安排工作量"));
        }
        let missing = (total(&d.task) - got).max(0) as u32;
        let reports: Vec<_> = p
            .unscheduled
            .iter()
            .filter(|u| u.task_id == d.task.id)
            .collect();
        if (missing == 0 && !reports.is_empty())
            || (missing > 0
                && (reports.len() != 1 || reports[0].minutes != missing.min(d.task.remaining)))
        {
            return Err(AppError::new(
                "work_conservation",
                "已安排与未安排工作量不一致",
            ));
        }
        let bs: Vec<_> = p.blocks.iter().filter(|b| b.task_id == d.task.id).collect();
        if !d.task.splittable && bs.len() > 1 {
            return Err(AppError::new("not_splittable", "任务不允许分段"));
        }
        for b in bs {
            if !b.active && b.end - b.start < ceil5(min_chunk(s, &d.task)).min(total(&d.task)) {
                return Err(AppError::new("minimum_chunk", "时间块短于最小连续时长"));
            }
        }
    }
    if p.unscheduled
        .iter()
        .any(|u| !s.demands.iter().any(|d| d.task.id == u.task_id))
    {
        return Err(AppError::new("unknown_task", "未安排列表包含未知任务"));
    }
    Ok(())
}

/// Recommendation only includes tasks executable in the current context.
pub fn recommend(s: &PlanningSnapshot) -> Vec<String> {
    let protected: Vec<_> = s
        .current
        .iter()
        .filter(|b| b.locked || b.active)
        .cloned()
        .collect();
    let mut tasks: Vec<_> = s
        .demands
        .iter()
        .filter(|d| {
            !d.task.blocked
                && d.earliest <= s.now
                && deps(s, &d.task)
                    .iter()
                    .all(|id| s.completed_ids.iter().any(|v| v == id))
        })
        .filter(|d| {
            let length = if d.task.splittable {
                ceil5(min_chunk(s, &d.task)).min(total(&d.task))
            } else {
                total(&d.task)
            };
            let others = protected
                .iter()
                .filter(|b| b.task_id != d.task.id)
                .cloned()
                .collect::<Vec<_>>();
            allowed(s, &d.task, s.now, s.now + length, &others)
        })
        .collect();
    tasks.sort_by_key(|d| (d.task.priority, d.task.deadline, d.task.id.clone()));
    tasks.into_iter().map(|d| d.task.id.clone()).collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    fn task(id: &str, minutes: u32, deps: Vec<&str>) -> WorkDemand {
        WorkDemand {
            earliest: 0,
            task: Task {
                id: id.into(),
                title: id.into(),
                project: "test".into(),
                category: "study".into(),
                remaining: minutes,
                estimate: minutes,
                deadline: 600,
                priority: 1,
                min_chunk: 5,
                splittable: true,
                conditions: Conditions::default(),
                dependencies: deps.into_iter().map(String::from).collect(),
                completed: false,
                blocked: false,
                completion_criteria: "done".into(),
                deferrals: 0,
                parent_id: None,
            },
        }
    }
    fn snap() -> PlanningSnapshot {
        PlanningSnapshot {
            version: 1,
            now: 0,
            until: 600,
            timezone: "UTC".into(),
            demands: vec![
                task("a", 60, vec![]),
                task("b", 60, vec!["a"]),
                task("c", 60, vec!["b"]),
            ],
            windows: vec![AvailabilityWindow {
                start: 0,
                end: 600,
                focus: 3,
                interruptible_only: false,
                equipment: vec![],
                location: "不限".into(),
            }],
            constraints: vec![],
            current: vec![],
            completed_ids: vec![],
        }
    }
    #[test]
    fn cascade_and_stability() {
        let mut s = snap();
        let first = propose(&s).unwrap();
        s.current = first.blocks.clone();
        assert_eq!(propose(&s).unwrap().blocks, s.current);
        s.demands[0].task.remaining = 150;
        let p = propose(&s).unwrap();
        validate(&s, &p).unwrap();
        assert!(p
            .blocks
            .iter()
            .filter(|b| b.task_id == "b")
            .all(|b| b.start >= 150));
        assert!(p
            .blocks
            .iter()
            .filter(|b| b.task_id == "c")
            .all(|b| b.start >= 210));
    }
    #[test]
    fn shortages_do_not_overlap_or_fake_completion() {
        let mut s = snap();
        s.windows[0].end = 90;
        let p = propose(&s).unwrap();
        assert!(p.unscheduled.iter().any(|u| u.task_id == "b"));
        assert!(p.blocks.iter().all(|b| b.task_id != "c"));
        validate(&s, &p).unwrap();
    }
    #[test]
    fn fixed_attention_cycle_and_unknown() {
        let mut s = snap();
        s.windows[0].focus = 1;
        assert_eq!(propose(&s).unwrap().blocks.len(), 0);
        s.windows[0].focus = 3;
        s.constraints.push(Constraint::Fixed {
            start: 0,
            end: 60,
            source: "life".into(),
            reason: "休息".into(),
        });
        assert!(propose(&s).unwrap().blocks.iter().all(|b| b.start >= 60));
        s.demands[0].task.dependencies.push("c".into());
        assert!(propose(&s).is_err());
        assert!(serde_json::from_str::<Constraint>(r#"{"type":"madeUp"}"#).is_err());
    }
    #[test]
    fn pinned_conflict_is_not_silently_moved() {
        let mut s = snap();
        s.current = vec![TimeBlock {
            id: "pinned".into(),
            task_id: "b".into(),
            start: 0,
            end: 60,
            locked: true,
            active: false,
        }];
        assert!(propose(&s).is_err());
    }
    #[test]
    fn training_gap_and_quota() {
        let mut s = snap();
        s.demands = vec![task("x", 60, vec![]), task("y", 60, vec![])];
        s.constraints.push(Constraint::MinimumGap {
            task_ids: vec!["x".into(), "y".into()],
            minutes: 120,
            source: "extension".into(),
            reason: "恢复".into(),
        });
        let p = propose(&s).unwrap();
        assert!(p.blocks[1].start - p.blocks[0].end >= 120);
    }
    #[test]
    fn benchmark_300_tasks() {
        let mut s = snap();
        s.until = 20160;
        s.windows[0].end = s.until;
        s.demands = (0..300)
            .map(|i| {
                let mut d = task(&format!("t{i:03}"), 30, vec![]);
                d.task.deadline = 20160;
                d
            })
            .collect();
        let start = std::time::Instant::now();
        let p = propose(&s).unwrap();
        assert_eq!(p.unscheduled.len(), 0);
        assert!(
            start.elapsed().as_secs_f64() < 3.0,
            "{}",
            start.elapsed().as_secs_f64()
        );
        eprintln!("300-task full replan: {:?}", start.elapsed());
    }
}
