use planner_contracts::*;
use planner_framework::UnitOfWork;
pub fn list(u: &UnitOfWork) -> Result<Vec<Notice>> {
    let mut items: Vec<Notice> = u.repo("reminders")?.all()?;
    items.sort_by_key(|n| std::cmp::Reverse(n.created_at));
    Ok(items)
}
pub fn add(u: &UnitOfWork, id: &str, title: &str, body: &str, now: Minute) -> Result<()> {
    if u.repo("reminders")?.get::<Notice>(id)?.is_none() {
        u.repo("reminders")?.put(
            id,
            &Notice {
                id: id.into(),
                title: title.into(),
                body: body.into(),
                read: false,
                created_at: now,
            },
        )?;
    }
    Ok(())
}
pub fn mark_read(u: &UnitOfWork, id: &str) -> Result<()> {
    if let Some(mut n) = u.repo("reminders")?.get::<Notice>(id)? {
        n.read = true;
        u.repo("reminders")?.put(id, &n)?
    }
    Ok(())
}
pub fn awaiting_feedback(
    u: &UnitOfWork,
    blocks: &[TimeBlock],
    executions: &[Execution],
    now: Minute,
) -> Result<()> {
    for b in blocks.iter().filter(|b| b.end <= now) {
        if !executions.iter().any(|e| {
            e.task_id == b.task_id
                && e.end.is_some_and(|end| end >= b.start)
                && e.outcome.as_deref() != Some("paused")
        }) {
            add(
                u,
                &format!("feedback:{}", b.id),
                "执行结果待确认",
                &format!(
                    "任务 {} 的计划时间已结束，请更新结果；系统不会自动将它标记完成。",
                    b.task_id
                ),
                now,
            )?;
        }
    }
    Ok(())
}
