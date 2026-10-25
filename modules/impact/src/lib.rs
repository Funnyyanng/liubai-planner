use planner_contracts::*;
use std::collections::{BTreeSet, VecDeque};
pub fn analyze(s: &PlanningSnapshot, trigger: &str, p: &PlanProposal) -> ImpactReport {
    let mut report = ImpactReport {
        trigger: trigger.into(),
        ..Default::default()
    };
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::new();
    if s.demands.iter().any(|d| d.task.id == trigger) {
        seen.insert(trigger.to_string());
        queue.push_back(trigger.to_string());
    }
    for c in &p.changes {
        if seen.insert(c.task_id.clone()) {
            queue.push_back(c.task_id.clone());
        }
    }
    while let Some(id) = queue.pop_front() {
        for d in &s.demands {
            if d.task.dependencies.contains(&id) {
                report.edges.push(ImpactEdge {
                    from: id.clone(),
                    to: d.task.id.clone(),
                    kind: "dependency".into(),
                    reason: "前置任务变化，后续任务需要重新检查".into(),
                });
                if seen.insert(d.task.id.clone()) {
                    queue.push_back(d.task.id.clone());
                }
            }
        }
        for b in p.blocks.iter().filter(|b| b.task_id == id) {
            for old in s
                .current
                .iter()
                .filter(|old| old.task_id != id && old.start < b.end && old.end > b.start)
            {
                report.edges.push(ImpactEdge {
                    from: id.clone(),
                    to: old.task_id.clone(),
                    kind: "competition".into(),
                    reason: "新安排占用了原执行时间".into(),
                });
                if seen.insert(old.task_id.clone()) {
                    queue.push_back(old.task_id.clone());
                }
            }
        }
    }
    report.affected = seen.into_iter().collect();
    report.risks = p.risks.clone();
    for u in &p.unscheduled {
        report.risks.push(format!(
            "{} 尚有 {} 分钟未安排：{}",
            s.demands
                .iter()
                .find(|d| d.task.id == u.task_id)
                .map(|d| d.task.title.as_str())
                .unwrap_or(&u.task_id),
            u.minutes,
            u.reason
        ));
    }
    report
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn competition_propagates_to_other_changed_blocks() {
        let block = |id: &str, task: &str, start, end| TimeBlock {
            id: id.into(),
            task_id: task.into(),
            start,
            end,
            locked: false,
            active: false,
        };
        let s = PlanningSnapshot {
            version: 1,
            now: 0,
            until: 300,
            timezone: "UTC".into(),
            demands: vec![],
            windows: vec![],
            constraints: vec![],
            current: vec![
                block("a", "a", 0, 60),
                block("b", "b", 60, 120),
                block("c", "c", 120, 180),
            ],
            completed_ids: vec![],
        };
        let p = PlanProposal {
            input_version: 1,
            blocks: vec![
                block("a2", "a", 0, 120),
                block("b2", "b", 120, 180),
                block("c2", "c", 180, 240),
            ],
            unscheduled: vec![],
            risks: vec![],
            requires_approval: false,
            changes: vec![PlanChange {
                task_id: "a".into(),
                before: vec![],
                after: vec![],
                reason: "delay".into(),
            }],
        };
        let report = analyze(&s, "a", &p);
        assert!(report.affected.contains(&"c".into()));
        assert!(report.edges.iter().any(|e| e.from == "b" && e.to == "c"));
    }
}
