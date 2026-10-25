import type {
  AppState,
  Attention,
  CalendarEvent,
  Task,
  TimeBlock,
} from "../types";
export type Page =
  | "canvas"
  | "now"
  | "calendar"
  | "projects"
  | "adjustments"
  | "training"
  | "modules"
  | "settings";
export type Modal =
  | { type: "task"; task?: Task }
  | { type: "event"; event?: CalendarEvent }
  | { type: "feedback"; task: Task }
  | { type: "block"; block: TimeBlock }
  | { type: "ai" }
  | { type: "notices" }
  | null;
export type PageContext = {
  data: AppState;
  page: Page;
  busy: boolean;
  today: number;
  date: string;
  tz: string;
  dayOffset: number;
  setDayOffset: React.Dispatch<React.SetStateAction<number>>;
  setModal: (m: Modal) => void;
  setPage: (p: Page) => void;
  query: string;
  setQuery: (s: string) => void;
  events: CalendarEvent[];
  blocks: TimeBlock[];
  scheduledMinutes: number;
  openTasks: Task[];
  risks: AppState["plan"] extends infer _T
    ? { taskId: string; minutes: number; reason: string }[]
    : never;
  active: AppState["executions"][number] | undefined;
  next: Task | undefined;
  name: (id: string) => string;
  changeAttention: (e: CalendarEvent, a: Attention) => Promise<void>;
  run: (
    name: string,
    payload?: unknown,
    message?: string,
  ) => Promise<AppState | undefined>;
  action: (name: string, args?: Record<string, unknown>) => Promise<void>;
  setToast: (s: string) => void;
};
