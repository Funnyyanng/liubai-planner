use serde::{Deserialize, Serialize};
pub const CONTRACT_VERSION: u32 = 1;
pub type Minute = i64;
pub type Result<T> = std::result::Result<T, AppError>;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
}
impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for AppError {}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Conditions {
    pub focus: u8,
    pub interruptible: bool,
    pub equipment: Vec<String>,
    pub location: Option<String>,
}
impl Default for Conditions {
    fn default() -> Self {
        Self {
            focus: 2,
            interruptible: true,
            equipment: vec![],
            location: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub project: String,
    pub category: String,
    pub remaining: u32,
    pub estimate: u32,
    pub deadline: Minute,
    pub priority: u8,
    pub min_chunk: u32,
    pub splittable: bool,
    pub conditions: Conditions,
    pub dependencies: Vec<String>,
    pub completed: bool,
    pub blocked: bool,
    pub completion_criteria: String,
    pub deferrals: u32,
    pub parent_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub start: Minute,
    pub end: Minute,
    pub location: String,
    pub equipment: Vec<String>,
    pub recurrence: Option<Recurrence>,
    pub source: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Recurrence {
    pub until: Minute,
    pub interval_weeks: u32,
    pub weekdays: Vec<u32>,
    pub excluded_dates: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Attention {
    Focused,
    Partial,
    Available,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextOverride {
    pub id: String,
    pub event_id: String,
    pub date: Option<String>,
    pub attention: Attention,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AvailabilityWindow {
    pub start: Minute,
    pub end: Minute,
    pub focus: u8,
    pub interruptible_only: bool,
    pub equipment: Vec<String>,
    pub location: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProtectedPeriod {
    pub id: String,
    pub title: String,
    pub start_hour: u32,
    pub end_hour: u32,
    pub weekdays: Vec<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LifeRules {
    pub timezone: String,
    pub day_start: u32,
    pub day_end: u32,
    pub buffer_percent: u32,
    pub equipment: Vec<String>,
    pub location: String,
    pub protected: Vec<ProtectedPeriod>,
}
impl Default for LifeRules {
    fn default() -> Self {
        Self {
            timezone: "Asia/Shanghai".into(),
            day_start: 8,
            day_end: 22,
            buffer_percent: 15,
            equipment: vec!["电脑".into()],
            location: "不限".into(),
            protected: vec![
                ProtectedPeriod {
                    id: "lunch".into(),
                    title: "午餐与休息".into(),
                    start_hour: 12,
                    end_hour: 13,
                    weekdays: (0..7).collect(),
                },
                ProtectedPeriod {
                    id: "leisure".into(),
                    title: "留给自己的时间".into(),
                    start_hour: 20,
                    end_hour: 21,
                    weekdays: (0..7).collect(),
                },
            ],
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkDemand {
    pub task: Task,
    pub earliest: Minute,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Constraint {
    Fixed {
        start: Minute,
        end: Minute,
        source: String,
        reason: String,
    },
    Allowed {
        task_id: String,
        start: Minute,
        end: Minute,
        source: String,
        reason: String,
    },
    Dependency {
        before: String,
        after: String,
        source: String,
        reason: String,
    },
    MinimumChunk {
        task_id: String,
        minutes: u32,
        source: String,
        reason: String,
    },
    Conditions {
        task_id: String,
        conditions: Conditions,
        source: String,
        reason: String,
    },
    MinimumGap {
        task_ids: Vec<String>,
        minutes: u32,
        source: String,
        reason: String,
    },
    Quota {
        task_ids: Vec<String>,
        start: Minute,
        end: Minute,
        minimum_minutes: u32,
        source: String,
        reason: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimeBlock {
    pub id: String,
    pub task_id: String,
    pub start: Minute,
    pub end: Minute,
    pub locked: bool,
    pub active: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanningSnapshot {
    pub version: u64,
    pub now: Minute,
    pub until: Minute,
    pub timezone: String,
    pub demands: Vec<WorkDemand>,
    pub windows: Vec<AvailabilityWindow>,
    pub constraints: Vec<Constraint>,
    pub current: Vec<TimeBlock>,
    pub completed_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unscheduled {
    pub task_id: String,
    pub minutes: u32,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanChange {
    pub task_id: String,
    pub before: Vec<TimeBlock>,
    pub after: Vec<TimeBlock>,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanProposal {
    pub input_version: u64,
    pub blocks: Vec<TimeBlock>,
    pub unscheduled: Vec<Unscheduled>,
    pub risks: Vec<String>,
    pub changes: Vec<PlanChange>,
    pub requires_approval: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpactEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImpactReport {
    pub trigger: String,
    pub affected: Vec<String>,
    pub edges: Vec<ImpactEdge>,
    pub risks: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Execution {
    pub id: String,
    pub task_id: String,
    pub start: Minute,
    pub end: Option<Minute>,
    pub outcome: Option<String>,
    pub remaining: Option<u32>,
    pub note: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecutionFeedback {
    pub task_id: String,
    pub outcome: String,
    pub remaining: u32,
    pub actual_minutes: u32,
    pub note: String,
    pub now: Minute,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanRecord {
    pub id: String,
    pub created_at: Minute,
    pub proposal: PlanProposal,
    pub impact: ImpactReport,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleManifest {
    pub id: String,
    pub version: String,
    pub contract_version: u32,
    pub dependencies: Vec<String>,
    pub optional_capabilities: Vec<String>,
    pub commands: Vec<String>,
    pub queries: Vec<String>,
    pub events: Vec<String>,
    pub pages: Vec<String>,
    pub critical: bool,
    pub migration_version: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleHealth {
    pub id: String,
    pub status: String,
    pub detail: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    pub id: String,
    pub title: String,
    pub body: String,
    pub read: bool,
    pub created_at: Minute,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiConfig {
    pub endpoint: String,
    pub model: String,
    pub enabled: bool,
    pub max_calls_per_day: u32,
}
impl Default for AiConfig {
    fn default() -> Self {
        Self {
            endpoint: "".into(),
            model: "".into(),
            enabled: false,
            max_calls_per_day: 20,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiDraft {
    pub input_version: u64,
    pub summary: String,
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub actions: Vec<AiAction>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum AiAction {
    Remaining {
        task_id: String,
        remaining: u32,
        note: String,
    },
    Attention {
        event_id: String,
        attention: Attention,
        date: Option<String>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrainingConfig {
    pub enabled: bool,
    pub sessions: u32,
    pub minutes: u32,
    pub gap_hours: u32,
    pub title: String,
}
impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sessions: 3,
            minutes: 60,
            gap_hours: 24,
            title: "力量训练".into(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub current_time: Minute,
    pub test_clock: bool,
    pub recommendations: Vec<String>,
    pub version: u64,
    pub tasks: Vec<Task>,
    pub calendar: Vec<CalendarEvent>,
    pub contexts: Vec<ContextOverride>,
    pub rules: LifeRules,
    pub executions: Vec<Execution>,
    pub plan: Option<PlanRecord>,
    pub history: Vec<PlanRecord>,
    pub pending: Option<PlanRecord>,
    pub notices: Vec<Notice>,
    pub modules: Vec<ModuleHealth>,
    pub ai: AiConfig,
    pub training: TrainingConfig,
    pub diagnostic: Option<String>,
}
