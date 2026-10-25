use planner_ai as ai;
use planner_calendar as calendar;
use planner_canvas as canvas;
use planner_context as context;
use planner_contracts::*;
use planner_execution as execution;
use planner_framework::{platform::ModelTransport, time, Store, UnitOfWork};
use planner_ics as ics;
use planner_impact as impact;
use planner_life as life;
use planner_plans as plans;
use planner_reminders as reminders;
use planner_scheduler as scheduler;
use planner_tasks as tasks;
use planner_training as training;
use serde_json::{json, Value};
use std::path::Path;
pub struct Service {
    pub store: Store,
    manifests: Vec<ModuleManifest>,
    runtime: planner_framework::registry::ModuleRuntime,
}
fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn decode<T: serde::de::DeserializeOwned>(v: Value) -> Result<T> {
    serde_json::from_value(v).map_err(|e| AppError::new("invalid_input", e.to_string()))
}
impl Service {
    pub fn open(path: &Path, manifests: Vec<ModuleManifest>) -> Result<Self> {
        use planner_framework::registry::{LifecycleHook, ModuleRuntime, Registration};
        planner_framework::registry::validate(&manifests)?;
        let store = Store::open(path, &manifests)?;
        let registrations = manifests
            .iter()
            .map(|m| {
                let initialize: LifecycleHook = match m.id.as_str() {
                    "tasks" => |u| tasks::list(u).and_then(|t| tasks::validate(&t)),
                    "calendar" => |u| calendar::list(u).map(|_| ()),
                    "canvas" => |u| canvas::cached(u, 0).map(|_| ()),
                    "context" => |u| context::list(u).map(|_| ()),
                    "execution" => |u| execution::list(u).map(|_| ()),
                    "life" => |u| life::get(u).map(|_| ()),
                    "plans" => |u| plans::current(u).map(|_| ()),
                    "ai" => |u| ai::config(u).map(|_| ()),
                    "training" => |u| training::get(u).map(|_| ()),
                    "reminders" => |u| reminders::list(u).map(|_| ()),
                    _ => |_| Ok(()),
                };
                Registration {
                    manifest: m.clone(),
                    initialize,
                    shutdown: |_| Ok(()),
                }
            })
            .collect();
        let runtime = ModuleRuntime::initialize(&store, registrations)?;
        Ok(Self {
            store,
            manifests,
            runtime,
        })
    }

    pub fn canvas_scan(&self) -> Result<Vec<canvas::Preview>> {
        canvas::scan(
            &planner_framework::external::CanvasReader::connect()?,
            time::now(),
        )
    }
    pub fn canvas_courses(&self) -> Result<Vec<canvas::Course>> {
        canvas::courses(&planner_framework::external::CanvasReader)
    }
    pub fn canvas_preview(&self, course_id: u64) -> Result<canvas::Preview> {
        let reader = planner_framework::external::CanvasReader::connect()?;
        let course = canvas::courses(&reader)?
            .into_iter()
            .find(|c| c.id == course_id)
            .ok_or_else(|| AppError::new("canvas_course", "课程不可访问"))?;
        let p = canvas::fetch(&reader, course, time::now())?;
        Ok(canvas::upcoming(p, time::now()))
    }
    pub fn canvas_import(
        &self,
        request: canvas::Import,
        preview: canvas::Preview,
        version: u64,
        command_id: &str,
    ) -> Result<AppState> {
        self.store.transact(Some(version), command_id, |u| {
            let changed = canvas::merge(&preview, &request, &tasks::list(u)?)?;
            tasks::apply_batch(u, &changed)?;
            if request.import_events {
                for event in &preview.events {
                    calendar::save(u, event)?;
                }
            }
            canvas::save(u, &preview)?;
            u.emit("canvas.synced", &request.course_id)
        })?;
        if let Err(e) = self.replan(time::now(), "Canvas 同步") {
            self.store
                .transact(None, &format!("canvas-plan-error:{}", id()), |u| {
                    u.repo("coordinator")?
                        .put("planning_error", &Some(e.message))
                })?;
        }
        self.state()
    }
    pub fn state(&self) -> Result<AppState> {
        self.store.read(|u| {
            let config = ai::config(u).unwrap_or_default();
            let training = training::get(u).unwrap_or_default();
            Ok(AppState {
                current_time: time::now(),
                test_clock: time::test_now().is_some(),
                recommendations: snapshot(u, time::now(), 14)
                    .map(|s| scheduler::recommend(&s))
                    .unwrap_or_default(),
                version: u.revision()?,
                tasks: tasks::list(u)?,
                calendar: calendar::list(u)?,
                contexts: context::list(u)?,
                rules: life::get(u)?,
                executions: execution::list(u)?,
                plan: plans::current(u)?,
                history: plans::history(u)?,
                pending: plans::pending(u)?,
                notices: reminders::list(u).unwrap_or_default(),
                modules: self
                    .runtime
                    .health
                    .iter()
                    .map(|m| ModuleHealth {
                        id: m.id.clone(),
                        status: if m.status == "failed" {
                            "failed"
                        } else if (m.id == "ai" && !config.enabled)
                            || (m.id == "training" && !training.enabled)
                        {
                            "disabled"
                        } else {
                            "ready"
                        }
                        .into(),
                        detail: if m.status == "failed" {
                            &m.detail
                        } else if m.id == "ai" && !config.enabled {
                            "未配置；本地规划仍可使用"
                        } else {
                            "契约与迁移已验证"
                        }
                        .into(),
                    })
                    .collect(),
                ai: config,
                training,
                diagnostic: u.repo("coordinator")?.get("planning_error")?.flatten(),
            })
        })
    }
    fn ensure_training(&self, now: Minute) -> Result<()> {
        let new = self.store.read(|u| {
            let r = life::get(u)?;
            let ts = tasks::list(u)?;
            Ok(
                training::contribute(&training::get(u).unwrap_or_default(), now, &r.timezone, &ts)?
                    .0,
            )
        })?;
        if !new.is_empty() {
            self.store
                .transact(None, &format!("training:{}", id()), |u| {
                    tasks::apply_batch(u, &new)
                })?;
        }
        Ok(())
    }
    pub fn snapshot(&self, now: Minute, days: u32) -> Result<PlanningSnapshot> {
        self.store.read(|u| snapshot(u, now, days))
    }
    pub fn replan(&self, now: Minute, trigger: &str) -> Result<()> {
        let events = self.store.pending_events()?;
        let result = self.replan_inner(now, trigger);
        if let Err(error) = &result {
            if error.code != "stale_version" {
                for event in events {
                    let _ = self.store.event_failed(event.id);
                }
            }
        }
        result
    }
    fn replan_inner(&self, now: Minute, trigger: &str) -> Result<()> {
        self.ensure_training(now)?;
        let running_over = self.store.read(|u| {
            let active = execution::list(u)?.into_iter().find(|e| e.end.is_none());
            let p = plans::current(u)?;
            Ok(active.is_some_and(|e| {
                p.is_some_and(|p| {
                    p.proposal
                        .blocks
                        .iter()
                        .any(|b| b.task_id == e.task_id && b.active && b.end <= now)
                })
            }))
        })?;
        if running_over {
            self.store
                .transact(None, &format!("overrun:{}", now), |u| {
                    let message = "当前专注时间已结束。请确认完成情况或剩余时间，再调整后续安排。";
                    u.repo("coordinator")?
                        .put("planning_error", &Some(message))?;
                    reminders::add(u, &format!("overrun:{now}"), "请更新执行结果", message, now)?;
                    Ok(())
                })?;
            return Ok(());
        }

        let events = self.store.pending_events()?;
        let event_ids: Vec<_> = events.iter().map(|e| e.id).collect();
        let full = self.snapshot(now, 14)?;
        let local = self.snapshot(now, 1)?;
        let full_p = scheduler::propose(&full);
        let proposal = match full_p {
            Ok(p) => p,
            Err(e) => {
                let signature = format!("conflict:{}:{}", full.version, e.code);
                self.store.transact(Some(full.version), &signature, |u| {
                    plans::invalidate_pending(u)?;
                    u.repo("coordinator")?
                        .put("planning_error", &Some(e.message.clone()))?;
                    reminders::add(u, &signature, "计划需要你处理", &e.message, now)?;
                    u.acknowledge(&event_ids)?;
                    Ok(())
                })?;
                return Ok(());
            }
        };
        // Full window is used if dependencies or capacity cannot be satisfied today.
        let mut proposal = if let Ok(local_p) = scheduler::propose(&local) {
            if local_p.unscheduled.is_empty() && full.current.iter().all(|b| b.end <= local.until) {
                let mut p = local_p;
                p.input_version = full.version;
                p
            } else {
                proposal
            }
        } else {
            proposal
        };
        let report = impact::analyze(&full, trigger, &proposal);
        let current = self.store.read(plans::current)?;
        if current.as_ref().is_some_and(|p| {
            p.proposal.blocks == proposal.blocks
                && serde_json::to_value(&p.proposal.unscheduled).ok()
                    == serde_json::to_value(&proposal.unscheduled).ok()
                && p.proposal.risks == proposal.risks
        }) {
            if !event_ids.is_empty() {
                self.store
                    .transact(Some(full.version), &format!("ack:{}", id()), |u| {
                        u.acknowledge(&event_ids)?;
                        u.repo("coordinator")?
                            .put("planning_error", &Option::<String>::None)
                    })?;
            }
            return Ok(());
        }
        proposal.requires_approval = false;
        let record = PlanRecord {
            id: id(),
            created_at: now,
            proposal,
            impact: report,
            reason: trigger.into(),
        };
        self.store
            .transact(Some(full.version), &format!("plan:{}", record.id), |u| {
                plans::apply(u, &record)?;
                u.repo("coordinator")?
                    .put("planning_error", &Option::<String>::None)?;
                if !record.impact.risks.is_empty() {
                    reminders::add(
                        u,
                        &format!("risk:{}", record.id),
                        "计划中有待解决的事项",
                        &record.impact.risks.join("\n"),
                        now,
                    )?;
                }
                reminders::awaiting_feedback(
                    u,
                    &current
                        .as_ref()
                        .map(|p| p.proposal.blocks.clone())
                        .unwrap_or_default(),
                    &execution::list(u)?,
                    now,
                )?;
                u.acknowledge(&event_ids)?;
                Ok(())
            })?;
        Ok(())
    }
    pub fn wake(&self, now: Minute) -> Result<()> {
        let needs = self.store.read(|u| {
            let previous: Option<Minute> = u.repo("coordinator")?.get("last_wake")?;
            Ok(previous.is_none_or(|last| now - last >= 1))
        })?;
        if needs && self.store.exhausted_events()? > 0 && self.store.pending_events()?.is_empty() {
            return Ok(());
        }
        if needs {
            self.store.transact(None, &format!("wake:{now}"), |u| {
                if let Some(p) = plans::current(u)? {
                    reminders::awaiting_feedback(u, &p.proposal.blocks, &execution::list(u)?, now)?;
                }
                u.repo("coordinator")?.put("last_wake", &now)
            })?;
            self.replan(now, "时间与执行状态检查")?;
        }
        Ok(())
    }
    pub fn command(
        &self,
        name: &str,
        payload: Value,
        expected: u64,
        command_id: &str,
        now: Minute,
    ) -> Result<AppState> {
        let required = match name {
            "configureAi" | "applyDraft" => Some("ai"),
            "configureTraining" => Some("training"),
            "importIcs" => Some("ics"),
            "markRead" => Some("reminders"),
            _ => None,
        };
        if required.is_some_and(|id| !self.runtime.available(id)) {
            return Err(AppError::new(
                "module_unavailable",
                "该可选模块初始化失败，请查看模块中心",
            ));
        }
        let mut replan = true;
        match name {
            "saveTask" => {
                let t: Task = decode(payload)?;
                self.store
                    .transact(Some(expected), command_id, |u| tasks::save(u, &t))?;
            }
            "saveEvent" => {
                let e: CalendarEvent = decode(payload)?;
                self.store
                    .transact(Some(expected), command_id, |u| calendar::save(u, &e))?;
            }
            "setAttention" => {
                let c: ContextOverride = decode(payload)?;
                self.store.transact(Some(expected), command_id, |u| {
                    if !calendar::list(u)?.iter().any(|e| e.id == c.event_id) {
                        return Err(AppError::new("event", "课程不存在"));
                    }
                    context::save(u, &c)
                })?;
            }
            "saveRules" => {
                let r: LifeRules = decode(payload)?;
                self.store
                    .transact(Some(expected), command_id, |u| life::save(u, &r))?;
            }
            "feedback" => {
                let mut f: ExecutionFeedback = decode(payload)?;
                f.now = now;
                self.store.transact(Some(expected), command_id, |u| {
                    execution::record(u, &f)?;
                    tasks::feedback(u, &f)?;
                    Ok(())
                })?;
                self.replan(now, &f.task_id)?;
                replan = false;
            }
            "start" => {
                let task_id = payload
                    .as_str()
                    .ok_or_else(|| AppError::new("task", "需要任务标识"))?;
                self.store.transact(Some(expected), command_id, |u| {
                    let ts = tasks::list(u)?;
                    let t = ts
                        .iter()
                        .find(|t| t.id == task_id)
                        .ok_or_else(|| AppError::new("task", "任务不存在"))?;
                    if t.completed
                        || t.blocked
                        || t.dependencies
                            .iter()
                            .any(|dep| !ts.iter().any(|x| &x.id == dep && x.completed))
                    {
                        return Err(AppError::new(
                            "task_blocked",
                            "任务未就绪，请先完成前置任务或解除阻碍",
                        ));
                    }
                    let mut s = snapshot(u, now, 14)?;
                    let duration = s
                        .current
                        .iter()
                        .find(|b| b.task_id == task_id)
                        .map(|b| b.end - b.start)
                        .unwrap_or(t.min_chunk as i64)
                        .min(t.remaining as i64);
                    s.current.retain(|b| b.task_id != task_id);
                    s.current.push(TimeBlock {
                        id: format!("active:{task_id}:{now}"),
                        task_id: task_id.into(),
                        start: now,
                        end: now + duration.max(5),
                        locked: false,
                        active: true,
                    });
                    let p = scheduler::propose(&s)?;
                    execution::start(u, task_id, now)?;
                    plans::apply(
                        u,
                        &PlanRecord {
                            id: id(),
                            created_at: now,
                            proposal: p,
                            impact: ImpactReport::default(),
                            reason: "开始执行".into(),
                        },
                    )?;
                    Ok(())
                })?;
            }
            "pause" => {
                self.store
                    .transact(Some(expected), command_id, |u| execution::pause(u, now))?;
            }
            "moveBlock" => {
                let b: TimeBlock = decode(payload)?;
                self.store.transact(Some(expected), command_id, |u| {
                    let mut s = snapshot(u, now, 14)?;
                    if s.current.iter().any(|old| old.id == b.id && old.active) {
                        return Err(AppError::new("active", "请先暂停正在执行的任务"));
                    }
                    s.current.retain(|x| x.id != b.id);
                    let mut proposed = b.clone();
                    proposed.locked = true;
                    s.current.push(proposed);
                    let mut p = scheduler::propose(&s)?;
                    if !b.locked {
                        for item in &mut p.blocks {
                            if item.id == b.id {
                                item.locked = false
                            }
                        }
                    }
                    plans::apply(
                        u,
                        &PlanRecord {
                            id: id(),
                            created_at: now,
                            impact: impact::analyze(&s, &b.task_id, &p),
                            proposal: p,
                            reason: "手动安排".into(),
                        },
                    )?;
                    Ok(())
                })?;
                replan = false;
            }
            "undo" => {
                self.store.transact(Some(expected), command_id, |u| {
                    let previous = plans::history(u)?
                        .first()
                        .cloned()
                        .ok_or_else(|| AppError::new("history", "没有可撤销的计划"))?;
                    let mut s = snapshot(u, now, 14)?;
                    let protected: Vec<_> = s
                        .current
                        .iter()
                        .filter(|b| b.locked || b.active)
                        .cloned()
                        .collect();
                    s.current = previous
                        .proposal
                        .blocks
                        .into_iter()
                        .filter(|b| b.end > now && s.demands.iter().any(|d| d.task.id == b.task_id))
                        .collect();
                    for b in &mut s.current {
                        b.start = b.start.max(now);
                    }
                    for b in protected {
                        s.current.retain(|old| old.id != b.id);
                        s.current.push(b);
                    }
                    let p = scheduler::propose(&s)?;
                    plans::apply(
                        u,
                        &PlanRecord {
                            id: id(),
                            created_at: now,
                            proposal: p,
                            impact: ImpactReport::default(),
                            reason: "恢复上版安排并按最新执行事实校验".into(),
                        },
                    )?;
                    Ok(())
                })?;
                replan = false;
            }
            "configureAi" => {
                let c: AiConfig = decode(payload)?;
                self.store
                    .transact(Some(expected), command_id, |u| ai::save(u, &c))?;
                replan = false;
            }
            "configureTraining" => {
                let c: TrainingConfig = decode(payload)?;
                self.store.transact(Some(expected), command_id, |u| {
                    training::save(u, &c)?;
                    let es = execution::list(u)?;
                    for mut t in tasks::list(u)?.into_iter().filter(|t| {
                        t.id.starts_with("training:") && !t.completed && t.deadline > now
                    }) {
                        let index =
                            t.id.rsplit(':')
                                .next()
                                .and_then(|v| v.parse::<u32>().ok())
                                .unwrap_or(0);
                        if index >= c.sessions {
                            t.blocked = true;
                        } else if !es.iter().any(|e| e.task_id == t.id) {
                            t.title = format!("{} · 第 {} 次", c.title, index + 1);
                            t.remaining = c.minutes;
                            t.estimate = c.minutes;
                            t.min_chunk = c.minutes;
                            t.blocked = false;
                        }
                        tasks::save(u, &t)?;
                    }
                    Ok(())
                })?;
            }
            "importIcs" => {
                let text = payload
                    .as_str()
                    .ok_or_else(|| AppError::new("ics", "需要文件内容"))?;
                let r = self.store.read(life::get)?;
                let events = ics::parse(text, &r.timezone)?;
                self.store.transact(Some(expected), command_id, |u| {
                    for e in &events {
                        calendar::save(u, e)?;
                    }
                    Ok(())
                })?;
            }
            "markRead" => {
                let notice = payload
                    .as_str()
                    .ok_or_else(|| AppError::new("notice", "缺少提醒标识"))?;
                self.store.transact(Some(expected), command_id, |u| {
                    reminders::mark_read(u, notice)
                })?;
                replan = false;
            }
            "applyDraft" => {
                let d: AiDraft = decode(payload)?;
                if d.input_version != expected {
                    return Err(AppError::new("stale_version", "AI 草案已过期，请重新生成"));
                }
                self.store.transact(Some(expected), command_id, |u| {
                    tasks::apply_batch(u, &d.tasks)?;
                    for action in &d.actions {
                        match action {
                            AiAction::Remaining {
                                task_id,
                                remaining,
                                note,
                            } => {
                                let mut t = tasks::list(u)?
                                    .into_iter()
                                    .find(|t| &t.id == task_id)
                                    .ok_or_else(|| AppError::new("task", "草案引用未知任务"))?;
                                if *remaining == 0 {
                                    return Err(AppError::new(
                                        "ai_scope",
                                        "请通过执行反馈确认完成，AI 不能自动完成任务",
                                    ));
                                }
                                t.remaining = *remaining;
                                t.blocked = false;
                                tasks::save(u, &t)?;
                                u.emit("ai.remaining", &json!({"taskId":task_id,"note":note}))?;
                            }
                            AiAction::Attention {
                                event_id,
                                attention,
                                date,
                            } => {
                                if !calendar::list(u)?.iter().any(|e| &e.id == event_id) {
                                    return Err(AppError::new("event", "草案引用未知日程"));
                                }
                                context::save(
                                    u,
                                    &ContextOverride {
                                        id: format!(
                                            "{event_id}:{}",
                                            date.as_deref().unwrap_or("all")
                                        ),
                                        event_id: event_id.clone(),
                                        date: date.clone(),
                                        attention: attention.clone(),
                                    },
                                )?;
                            }
                        }
                    }
                    Ok(())
                })?;
            }
            "replan" => {
                self.store
                    .transact(Some(expected), command_id, |u| u.retry_events())?;
            }
            "seedDemo" => {
                self.seed(now, expected, command_id)?;
            }
            _ => return Err(AppError::new("unknown_command", name)),
        }
        if replan {
            self.replan(now, name)?;
        }
        self.state()
    }
    pub fn draft(
        &self,
        transport: &dyn ModelTransport,
        prompt: &str,
        task_ids: &[String],
        event_ids: &[String],
        now: Minute,
    ) -> Result<AiDraft> {
        let c = self
            .store
            .transact(None, &format!("ai:{}", id()), |u| ai::reserve(u, now))?;
        let s = self.state()?;
        let ts: Vec<_> = s
            .tasks
            .into_iter()
            .filter(|t| task_ids.contains(&t.id))
            .collect();
        let es: Vec<_> = s
            .calendar
            .into_iter()
            .filter(|e| event_ids.contains(&e.id))
            .collect();
        let draft = ai::generate(transport, &c, s.version, prompt, &ts, &es, now)?;
        let mut all = self.store.read(tasks::list)?;
        all.extend(draft.tasks.clone());
        tasks::validate(&all)?;
        if self.state()?.version != draft.input_version {
            return Err(AppError::new(
                "stale_version",
                "生成期间数据已更新，请重新生成草案",
            ));
        }
        Ok(draft)
    }
    pub fn occurrences(&self, from: Minute, until: Minute) -> Result<Vec<CalendarEvent>> {
        self.store
            .read(|u| calendar::expand(&calendar::list(u)?, from, until, &life::get(u)?.timezone))
    }
    pub fn export_ics(&self) -> Result<String> {
        let s = self.state()?;
        Ok(ics::export(
            &s.plan.map(|p| p.proposal.blocks).unwrap_or_default(),
            &s.tasks,
        ))
    }
    pub fn restore(&self, path: &Path) -> Result<()> {
        self.store.restore(path, &self.manifests)
    }
    fn seed(&self, now: Minute, version: u64, command_id: &str) -> Result<()> {
        self.store.transact(Some(version), command_id, |u| {
            if !tasks::list(u)?.is_empty() || !calendar::list(u)?.is_empty() {
                return Err(AppError::new("demo", "体验数据仅能添加到空工作区"));
            }
            let r = life::get(u)?;
            let days = time::days(now, 3, &r.timezone)?;
            let day = if time::day_hour(days[0].0, 18, &r.timezone)? > now {
                days[0].0
            } else {
                days[1].0
            };
            let deadline = time::day_hour(days[2].0, 19, &r.timezone)?;
            let make = |id: &str,
                        title: &str,
                        project: &str,
                        cat: &str,
                        remaining: u32,
                        priority: u8,
                        deps: Vec<String>| Task {
                id: id.into(),
                title: title.into(),
                project: project.into(),
                category: cat.into(),
                remaining,
                estimate: remaining,
                deadline,
                priority,
                min_chunk: 25,
                splittable: true,
                conditions: Conditions::default(),
                dependencies: deps,
                completed: false,
                blocked: false,
                completion_criteria: "完成并检查结果".into(),
                deferrals: 0,
                parent_id: None,
            };
            let mut ts = vec![
                make(
                    "experiment",
                    "完成实验与数据采集",
                    "课题研究",
                    "research",
                    60,
                    0,
                    vec![],
                ),
                make(
                    "analysis",
                    "整理实验数据，完成初步分析",
                    "课题研究",
                    "research",
                    60,
                    0,
                    vec!["experiment".into()],
                ),
                make(
                    "report",
                    "撰写课题阶段报告",
                    "课题研究",
                    "research",
                    60,
                    1,
                    vec!["analysis".into()],
                ),
                make(
                    "assignment",
                    "完成线性代数作业",
                    "课程学习",
                    "study",
                    45,
                    1,
                    vec![],
                ),
                make("internship", "整理实习周报", "实习", "work", 30, 2, vec![]),
                make(
                    "reading",
                    "阅读两篇参考文献",
                    "课题研究",
                    "research",
                    30,
                    2,
                    vec![],
                ),
            ];
            ts[5].conditions.focus = 1;
            tasks::apply_batch(u, &ts)?;
            let start = time::day_hour(day, 10, &r.timezone)?;
            calendar::save(
                u,
                &CalendarEvent {
                    id: "course-demo".into(),
                    title: "计算机网络".into(),
                    start,
                    end: start + 90,
                    location: "教学楼 A203".into(),
                    equipment: vec!["电脑".into()],
                    recurrence: None,
                    source: "manual".into(),
                },
            )?;
            context::save(
                u,
                &ContextOverride {
                    id: "course-demo:all".into(),
                    event_id: "course-demo".into(),
                    date: None,
                    attention: Attention::Partial,
                },
            )?;
            Ok(())
        })
    }
}
impl Drop for Service {
    fn drop(&mut self) {
        self.runtime.shutdown(&self.store);
    }
}
fn snapshot(u: &UnitOfWork, now: Minute, days: u32) -> Result<PlanningSnapshot> {
    let rules = life::get(u)?;
    let all = tasks::list(u)?;
    let tc = training::get(u).unwrap_or_default();
    let until = time::day_hour(
        time::days(now, days, &rules.timezone)?.last().unwrap().0,
        24,
        &rules.timezone,
    )?;
    let ev = calendar::expand(&calendar::list(u)?, now, until, &rules.timezone)?;
    let windows = context::windows(
        &life::availability(&rules, now, days)?,
        &ev,
        &context::list(u)?,
        &rules.timezone,
    )?;
    let windows = life::reserve_buffer(&rules, &windows)?;
    let (_, mut constraints) = training::contribute(&tc, now, &rules.timezone, &all)?;
    let completed_ids: Vec<_> = all
        .iter()
        .filter(|t| t.completed)
        .map(|t| t.id.clone())
        .collect();
    // Completed training sessions contribute quota credit and protect recovery intervals.
    let executions = execution::list(u)?;
    for c in &mut constraints {
        if let Constraint::Quota {
            task_ids,
            minimum_minutes,
            start,
            end,
            ..
        } = c
        {
            let credit: u32 = all
                .iter()
                .filter(|t| t.completed && task_ids.contains(&t.id))
                .filter(|t| {
                    executions.iter().any(|e| {
                        e.task_id == t.id
                            && e.outcome.as_deref() == Some("completed")
                            && e.end.is_some_and(|v| v >= *start && v <= *end)
                    })
                })
                .map(|t| t.estimate)
                .sum();
            *minimum_minutes = minimum_minutes.saturating_sub(credit);
        }
    }
    let mut demands: Vec<_> = all
        .iter()
        .filter(|t| {
            !t.completed
                && t.remaining > 0
                && (!t.id.starts_with("training:")
                    || tc.enabled
                        && t.deadline > now
                        && t.id
                            .rsplit(':')
                            .next()
                            .and_then(|v| v.parse::<u32>().ok())
                            .is_some_and(|i| i < tc.sessions))
        })
        .map(|t| WorkDemand {
            task: t.clone(),
            earliest: now,
        })
        .collect();
    for d in &mut demands {
        if d.task.id.starts_with("training:") {
            let last = executions
                .iter()
                .filter(|e| {
                    e.task_id.starts_with("training:")
                        && e.task_id != d.task.id
                        && e.outcome.as_deref() == Some("completed")
                })
                .filter_map(|e| e.end)
                .max();
            if let Some(end) = last {
                d.earliest = d.earliest.max(end + tc.gap_hours as i64 * 60)
            }
        }
    }
    let known: std::collections::BTreeSet<_> = demands
        .iter()
        .map(|d| d.task.id.clone())
        .chain(completed_ids.iter().cloned())
        .collect();
    constraints.retain_mut(|c| match c {
        Constraint::Allowed { task_id, .. } => known.contains(task_id),
        Constraint::MinimumGap { task_ids, .. } | Constraint::Quota { task_ids, .. } => {
            task_ids.retain(|id| known.contains(id));
            !task_ids.is_empty()
        }
        _ => true,
    });
    let active = executions.iter().find(|e| e.end.is_none());
    let mut current = plans::current(u)?
        .map(|p| p.proposal.blocks)
        .unwrap_or_default();
    current.retain(|b| b.end > now && demands.iter().any(|d| d.task.id == b.task_id));
    for b in &mut current {
        b.active = active.is_some_and(|a| a.task_id == b.task_id && b.start <= now);
        b.start = b.start.max(now);
    }
    Ok(PlanningSnapshot {
        version: u.revision()?,
        now,
        until,
        timezone: rules.timezone,
        demands,
        windows,
        constraints,
        current,
        completed_ids,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canvas_import_is_atomic_idempotent_and_version_checked() {
        let (_dir, s) = setup();
        let preview = canvas::Preview {
            course: canvas::Course {
                id: 7,
                name: "课程".into(),
            },
            assignments: vec![canvas::Assignment {
                id: 9,
                title: "作业".into(),
                url: "https://oc.sjtu.edu.cn/courses/7/assignments/9".into(),
                due: Some(time::now() + 1440),
                submitted: true,
            }],
            events: vec![],
            resources: vec![],
            undated: vec![],
            warnings: vec![],
        };
        let request = canvas::Import {
            course_id: 7,
            assignment_ids: vec![9],
            minutes: 60,
            import_events: false,
        };
        let version = s.state().unwrap().version;
        let state = s
            .canvas_import(request.clone(), preview.clone(), version, "canvas-once")
            .unwrap();
        assert_eq!(state.tasks.len(), 1);
        assert!(!state.tasks[0].completed);
        s.canvas_import(request.clone(), preview.clone(), version, "canvas-once")
            .unwrap();
        assert_eq!(s.state().unwrap().tasks.len(), 1);
        assert!(s
            .canvas_import(request.clone(), preview.clone(), version, "canvas-stale")
            .is_err());
        let mut invalid = preview;
        invalid.events.push(CalendarEvent {
            id: "bad".into(),
            title: "".into(),
            start: 10,
            end: 5,
            location: "".into(),
            equipment: vec![],
            recurrence: None,
            source: "canvas".into(),
        });
        invalid.assignments[0].due = Some(time::now() + 2880);
        let before = s.state().unwrap();
        let mut request = request;
        request.import_events = true;
        assert!(s
            .canvas_import(request, invalid, before.version, "canvas-rollback")
            .is_err());
        assert_eq!(s.state().unwrap().tasks, before.tasks);
    }
    fn setup() -> (tempfile::TempDir, Service) {
        let dir = tempfile::tempdir().unwrap();
        let ms: Vec<ModuleManifest> =
            serde_json::from_str(include_str!("../../../modules.json")).unwrap();
        let s = Service::open(&dir.path().join("db.sqlite"), ms).unwrap();
        (dir, s)
    }
    #[test]
    fn full_feedback_chain_persists_and_rejects_stale() {
        let (_dir, s) = setup();
        let now = 29812800;
        let v = s.state().unwrap().version;
        let state = s.command("seedDemo", Value::Null, v, "seed", now).unwrap();
        assert_eq!(state.tasks.len(), 6);
        let version = state.version;
        let f = ExecutionFeedback {
            task_id: "experiment".into(),
            outcome: "unfinished".into(),
            remaining: 150,
            actual_minutes: 60,
            note: "实验超时".into(),
            now,
        };
        let state = s
            .command(
                "feedback",
                serde_json::to_value(f).unwrap(),
                version,
                "feedback-one",
                now,
            )
            .unwrap();
        assert_eq!(
            state
                .tasks
                .iter()
                .find(|t| t.id == "experiment")
                .unwrap()
                .remaining,
            150
        );
        assert!(
            !state
                .tasks
                .iter()
                .find(|t| t.id == "experiment")
                .unwrap()
                .completed
        );
        assert!(state
            .plan
            .unwrap()
            .impact
            .affected
            .contains(&"analysis".into()));
        assert!(s
            .command("saveTask", json!({}), version, "stale", now)
            .is_err());
        let again=s.command("feedback",json!({"taskId":"experiment","outcome":"unfinished","remaining":150,"actualMinutes":60,"note":"实验超时","now":now}),version,"feedback-one",now).unwrap();
        assert_eq!(again.executions.len(), 1);
    }
    #[test]
    fn importer_atomic_and_context_preserved() {
        let (_d, s) = setup();
        let text="BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:x\nDTSTART:20260907T010000Z\nDTEND:20260907T020000Z\nSUMMARY:课程\nEND:VEVENT\nEND:VCALENDAR";
        let a = s
            .command("importIcs", json!(text), 0, "i1", 29812800)
            .unwrap();
        let b = s
            .command(
                "setAttention",
                json!({"id":"ics:x:all","eventId":"ics:x","date":null,"attention":"available"}),
                a.version,
                "ctx",
                29812800,
            )
            .unwrap();
        let c = s
            .command("importIcs", json!(text), b.version, "i2", 29812800)
            .unwrap();
        assert_eq!(c.calendar.len(), 1);
        assert_eq!(c.contexts[0].attention, Attention::Available);
    }
    #[test]
    fn training_can_be_added_disabled_and_reconfigured_without_engine_changes() {
        let (_dir, s) = setup();
        let now = 29812800;
        let a = s
            .command(
                "configureTraining",
                json!({"enabled":true,"sessions":3,"minutes":60,"gapHours":24,"title":"训练"}),
                0,
                "training-on",
                now,
            )
            .unwrap();
        assert!(a.tasks.iter().any(|t| t.id.starts_with("training:")));
        assert!(a.diagnostic.is_none(), "{:?}", a.diagnostic);
        let b = s
            .command(
                "configureTraining",
                json!({"enabled":true,"sessions":2,"minutes":45,"gapHours":24,"title":"训练"}),
                a.version,
                "training-change",
                now,
            )
            .unwrap();
        assert!(b.diagnostic.is_none(), "{:?}", b.diagnostic);
        assert!(s
            .snapshot(now, 14)
            .unwrap()
            .demands
            .iter()
            .all(|d| !d.task.id.ends_with(":2")));
        let c = s
            .command(
                "configureTraining",
                json!({"enabled":false,"sessions":2,"minutes":45,"gapHours":24,"title":"训练"}),
                b.version,
                "training-off",
                now,
            )
            .unwrap();
        assert!(c.plan.unwrap().proposal.blocks.is_empty());
        assert!(!c.tasks.is_empty());
    }
    #[test]
    fn feedback_is_atomic_and_stale_ai_drafts_cannot_apply() {
        let (_dir, s) = setup();
        let now = 29812800;
        let state = s.command("seedDemo", Value::Null, 0, "seed", now).unwrap();
        let invalid = json!({"taskId":"missing","outcome":"unfinished","remaining":30,"actualMinutes":30,"note":"","now":now});
        assert!(s
            .command("feedback", invalid, state.version, "invalid", now)
            .is_err());
        assert!(s.state().unwrap().executions.is_empty());
        let d = AiDraft {
            input_version: state.version,
            summary: "test".into(),
            tasks: vec![],
            actions: vec![],
        };
        let changed = s
            .command(
                "saveRules",
                serde_json::to_value(state.rules).unwrap(),
                state.version,
                "rules",
                now,
            )
            .unwrap();
        assert!(s
            .command(
                "applyDraft",
                serde_json::to_value(d).unwrap(),
                changed.version,
                "old-ai",
                now
            )
            .is_err());
    }
    #[test]
    fn timer_expiry_requires_feedback_instead_of_auto_completion() {
        let (_dir, s) = setup();
        let now = chrono::DateTime::parse_from_rfc3339("2026-09-07T09:00:00+08:00")
            .unwrap()
            .timestamp()
            / 60;
        let a = s.command("seedDemo", Value::Null, 0, "seed", now).unwrap();
        let b = s
            .command("start", json!("experiment"), a.version, "start", now)
            .unwrap();
        let end = b
            .plan
            .as_ref()
            .unwrap()
            .proposal
            .blocks
            .iter()
            .find(|b| b.active)
            .unwrap()
            .end;
        s.wake(end + 1).unwrap();
        let c = s.state().unwrap();
        assert!(
            !c.tasks
                .iter()
                .find(|t| t.id == "experiment")
                .unwrap()
                .completed
        );
        assert!(c.diagnostic.is_some());
        assert!(c.executions.iter().any(|e| e.end.is_none()));
        let d=s.command("feedback",json!({"taskId":"experiment","outcome":"completed","remaining":0,"actualMinutes":60,"note":"","now":end+1}),c.version,"finish",end+1).unwrap();
        assert!(
            d.tasks
                .iter()
                .find(|t| t.id == "experiment")
                .unwrap()
                .completed
        );
        assert!(d.executions.iter().all(|e| e.end.is_some()));
    }
    #[test]
    fn backup_restore_never_reuses_an_old_input_version() {
        let (dir, s) = setup();
        let now = 29812800;
        let a = s.command("seedDemo", Value::Null, 0, "seed", now).unwrap();
        let path = dir.path().join("backup.sqlite");
        s.store.backup(&path).unwrap();
        let b = s
            .command(
                "saveRules",
                serde_json::to_value(a.rules).unwrap(),
                a.version,
                "edit",
                now,
            )
            .unwrap();
        s.restore(&path).unwrap();
        let c = s.state().unwrap();
        assert!(c.version > b.version);
        assert_eq!(c.tasks.len(), 6);
    }
}
