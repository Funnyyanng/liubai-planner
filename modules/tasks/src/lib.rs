use planner_contracts::*;
use planner_framework::UnitOfWork;
use std::collections::{BTreeMap, BTreeSet};
pub fn list(u: &UnitOfWork) -> Result<Vec<Task>> {
    u.repo("tasks")?.all()
}
pub fn save(u: &UnitOfWork, task: &Task) -> Result<()> {
    let mut tasks = list(u)?;
    tasks.retain(|t| t.id != task.id);
    tasks.push(task.clone());
    validate(&tasks)?;
    u.repo("tasks")?.put(&task.id, task)?;
    u.emit("task.changed", &task.id)
}
pub fn validate(tasks: &[Task]) -> Result<()> {
    let mut map = BTreeMap::new();
    for t in tasks {
        if t.id.is_empty()
            || t.title.trim().is_empty()
            || t.priority > 3
            || t.min_chunk == 0
            || t.conditions.focus > 3
            || t.remaining > 100000
            || t.estimate > 100000
        {
            return Err(AppError::new(
                "invalid_task",
                "任务标题、优先级或用时不合法",
            ));
        }
        if t.completed && t.remaining != 0 {
            return Err(AppError::new("invalid_task", "已完成任务不能有剩余工作量"));
        }
        if map.insert(&t.id, t).is_some() {
            return Err(AppError::new("duplicate_task", &t.id));
        }
    }
    fn visit<'a>(
        id: &'a String,
        map: &BTreeMap<&'a String, &'a Task>,
        seen: &mut BTreeSet<&'a String>,
        stack: &mut BTreeSet<&'a String>,
    ) -> Result<()> {
        if seen.contains(id) {
            return Ok(());
        }
        if !stack.insert(id) {
            return Err(AppError::new("dependency_cycle", "任务存在循环依赖"));
        }
        let t = map
            .get(id)
            .ok_or_else(|| AppError::new("missing_task", format!("缺少前置任务 {id}")))?;
        for d in &t.dependencies {
            visit(d, map, seen, stack)?
        }
        if let Some(p) = &t.parent_id {
            if p == id || !map.contains_key(p) {
                return Err(AppError::new("invalid_parent", "父任务不存在或引用自身"));
            }
        }
        stack.remove(id);
        seen.insert(id);
        Ok(())
    }
    for id in map.keys() {
        visit(id, &map, &mut BTreeSet::new(), &mut BTreeSet::new())?
    }
    Ok(())
}
pub fn feedback(u: &UnitOfWork, f: &ExecutionFeedback) -> Result<Task> {
    let mut task: Task = u
        .repo("tasks")?
        .get(&f.task_id)?
        .ok_or_else(|| AppError::new("missing_task", "任务不存在"))?;
    match f.outcome.as_str() {
        "completed" => {
            task.completed = true;
            task.remaining = 0;
            task.blocked = false
        }
        "unfinished" | "blocked" | "notStarted" => {
            if f.remaining == 0 {
                return Err(AppError::new("remaining", "未完成任务需要填写剩余时间"));
            }
            task.completed = false;
            task.remaining = f.remaining;
            task.blocked = f.outcome == "blocked";
            task.deferrals += 1
        }
        _ => return Err(AppError::new("outcome", "未知执行结果")),
    }
    save(u, &task)?;
    Ok(task)
}
pub fn apply_batch(u: &UnitOfWork, new: &[Task]) -> Result<()> {
    let old = list(u)?;
    let ids: BTreeSet<_> = old.iter().map(|t| &t.id).collect();
    if new.iter().any(|t| ids.contains(&t.id)) {
        return Err(AppError::new("duplicate_task", "新任务标识与现有任务冲突"));
    }
    let mut all = old;
    all.extend_from_slice(new);
    validate(&all)?;
    for t in new {
        u.repo("tasks")?.put(&t.id, t)?;
    }
    u.emit(
        "task.changed",
        &new.iter().map(|t| &t.id).collect::<Vec<_>>(),
    )
}
