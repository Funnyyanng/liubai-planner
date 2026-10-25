use planner_contracts::*;
use planner_framework::UnitOfWork;
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Data {
    current: Option<PlanRecord>,
    history: Vec<PlanRecord>,
    pending: Option<PlanRecord>,
}
fn get(u: &UnitOfWork) -> Result<Data> {
    Ok(u.repo("plans")?.get("data")?.unwrap_or_default())
}
pub fn current(u: &UnitOfWork) -> Result<Option<PlanRecord>> {
    Ok(get(u)?.current)
}
pub fn history(u: &UnitOfWork) -> Result<Vec<PlanRecord>> {
    Ok(get(u)?.history)
}
pub fn pending(u: &UnitOfWork) -> Result<Option<PlanRecord>> {
    Ok(get(u)?.pending)
}
pub fn stage(u: &UnitOfWork, p: &PlanRecord) -> Result<()> {
    let mut d = get(u)?;
    d.pending = Some(p.clone());
    u.repo("plans")?.put("data", &d)
}
pub fn apply(u: &UnitOfWork, p: &PlanRecord) -> Result<()> {
    let mut d = get(u)?;
    if let Some(old) = d.current.take() {
        d.history.insert(0, old);
        d.history.truncate(50);
    }
    d.current = Some(p.clone());
    d.pending = None;
    u.repo("plans")?.put("data", &d)?;
    u.emit("plan.changed", &p.id)
}
pub fn invalidate_pending(u: &UnitOfWork) -> Result<()> {
    let mut d = get(u)?;
    d.pending = None;
    u.repo("plans")?.put("data", &d)
}
