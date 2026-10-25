use planner_contracts::*;
use planner_framework::UnitOfWork;
pub fn list(u: &UnitOfWork) -> Result<Vec<Execution>> {
    u.repo("execution")?.all()
}
pub fn start(u: &UnitOfWork, task_id: &str, now: Minute) -> Result<()> {
    if list(u)?.iter().any(|x| x.end.is_none()) {
        return Err(AppError::new("active_execution", "请先暂停或完成当前任务"));
    }
    let e = Execution {
        id: uuid::Uuid::new_v4().to_string(),
        task_id: task_id.into(),
        start: now,
        end: None,
        outcome: None,
        remaining: None,
        note: String::new(),
    };
    u.repo("execution")?.put(&e.id, &e)?;
    u.emit("execution.started", &task_id)
}
pub fn pause(u: &UnitOfWork, now: Minute) -> Result<()> {
    for mut e in list(u)?.into_iter().filter(|e| e.end.is_none()) {
        e.end = Some(now.max(e.start));
        e.outcome = Some("paused".into());
        u.repo("execution")?.put(&e.id, &e)?;
    }
    u.emit("execution.paused", &now)
}
pub fn record(u: &UnitOfWork, f: &ExecutionFeedback) -> Result<()> {
    let active = list(u)?
        .into_iter()
        .find(|e| e.task_id == f.task_id && e.end.is_none());
    let mut e = active.unwrap_or(Execution {
        id: uuid::Uuid::new_v4().to_string(),
        task_id: f.task_id.clone(),
        start: f.now - f.actual_minutes as i64,
        end: None,
        outcome: None,
        remaining: None,
        note: String::new(),
    });
    e.end = Some(f.now.max(e.start));
    e.outcome = Some(f.outcome.clone());
    e.remaining = Some(f.remaining);
    e.note = f.note.clone();
    u.repo("execution")?.put(&e.id, &e)?;
    u.emit("execution.recorded", f)
}
