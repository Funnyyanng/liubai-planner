export type Conditions = {
  focus: number;
  interruptible: boolean;
  equipment: string[];
  location: string | null;
};
export type Task = {
  id: string;
  title: string;
  project: string;
  category: string;
  remaining: number;
  estimate: number;
  deadline: number;
  priority: number;
  minChunk: number;
  splittable: boolean;
  conditions: Conditions;
  dependencies: string[];
  completed: boolean;
  blocked: boolean;
  completionCriteria: string;
  deferrals: number;
  parentId: string | null;
};
export type CalendarEvent = {
  id: string;
  title: string;
  start: number;
  end: number;
  location: string;
  equipment: string[];
  recurrence: {
    until: number;
    intervalWeeks: number;
    weekdays: number[];
    excludedDates: string[];
  } | null;
  source: string;
};
export type Attention = "focused" | "partial" | "available";
export type ContextOverride = {
  id: string;
  eventId: string;
  date: string | null;
  attention: Attention;
};
export type LifeRules = {
  timezone: string;
  dayStart: number;
  dayEnd: number;
  bufferPercent: number;
  equipment: string[];
  location: string;
  protected: {
    id: string;
    title: string;
    startHour: number;
    endHour: number;
    weekdays: number[];
  }[];
};
export type TimeBlock = {
  id: string;
  taskId: string;
  start: number;
  end: number;
  locked: boolean;
  active: boolean;
};
export type Execution = {
  id: string;
  taskId: string;
  start: number;
  end: number | null;
  outcome: string | null;
  remaining: number | null;
  note: string;
};
export type PlanProposal = {
  inputVersion: number;
  blocks: TimeBlock[];
  unscheduled: { taskId: string; minutes: number; reason: string }[];
  risks: string[];
  changes: {
    taskId: string;
    before: TimeBlock[];
    after: TimeBlock[];
    reason: string;
  }[];
  requiresApproval: boolean;
};
export type ImpactReport = {
  trigger: string;
  affected: string[];
  edges: { from: string; to: string; kind: string; reason: string }[];
  risks: string[];
};
export type PlanRecord = {
  id: string;
  createdAt: number;
  proposal: PlanProposal;
  impact: ImpactReport;
  reason: string;
};
export type AiConfig = {
  endpoint: string;
  model: string;
  enabled: boolean;
  maxCallsPerDay: number;
};
export type AiDraft = {
  inputVersion: number;
  summary: string;
  tasks: Task[];
  actions: (
    | { type: "remaining"; task_id: string; remaining: number; note: string }
    | {
        type: "attention";
        event_id: string;
        attention: Attention;
        date: string | null;
      }
  )[];
};
export type TrainingConfig = {
  enabled: boolean;
  sessions: number;
  minutes: number;
  gapHours: number;
  title: string;
};
export type AppState = {
  currentTime?: number;
  testClock?: boolean;
  recommendations: string[];
  version: number;
  tasks: Task[];
  calendar: CalendarEvent[];
  contexts: ContextOverride[];
  rules: LifeRules;
  executions: Execution[];
  plan: PlanRecord | null;
  history: PlanRecord[];
  pending: PlanRecord | null;
  notices: {
    id: string;
    title: string;
    body: string;
    read: boolean;
    createdAt: number;
  }[];
  modules: { id: string; status: string; detail: string }[];
  ai: AiConfig;
  training: TrainingConfig;
  diagnostic: string | null;
};
