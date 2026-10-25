import type { CalendarEvent } from "./types";
let simulatedMinute: number | undefined;
export const setClock = (value?: number) => {
  simulatedMinute = value;
};
export const minute = () => simulatedMinute ?? Math.floor(Date.now() / 60000);
export const uid = () => crypto.randomUUID();
export const duration = (n: number) =>
  n < 60
    ? `${n} 分钟`
    : `${Math.floor(n / 60)} 小时${n % 60 ? ` ${n % 60} 分钟` : ""}`;
export const clock = (m: number, tz = "Asia/Shanghai") =>
  new Intl.DateTimeFormat("zh-CN", {
    timeZone: tz,
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
  }).format(new Date(m * 60000));
export const dayKey = (m: number, tz = "Asia/Shanghai") =>
  new Intl.DateTimeFormat("en-CA", {
    timeZone: tz,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(new Date(m * 60000));
export const datetimeInput = (m: number) => {
  const d = new Date(m * 60000);
  const pad = (v: number) => String(v).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
};
export const inputMinute = (v: string) =>
  Math.floor(new Date(v).getTime() / 60000);
export const categoryNames: Record<string, string> = {
  study: "课程学习",
  research: "课题研究",
  work: "实习工作",
  fitness: "训练健身",
  leisure: "休闲娱乐",
  personal: "个人事务",
};
export const moduleNames: Record<string, string> = {
  tasks: "任务与项目",
  calendar: "日历",
  context: "情境与可用性",
  execution: "执行记录",
  life: "生活规则",
  impact: "影响分析",
  scheduler: "排程引擎",
  plans: "计划管理",
  coordinator: "规划协调",
  ai: "AI 辅助",
  ics: "日历导入导出",
  reminders: "提醒",
  training: "训练扩展",
};
export const errorText = (e: unknown) =>
  typeof e === "object" && e !== null && "message" in e
    ? String(e.message)
    : String(e);
// Calendar expansion is performed by the backend for the desktop timeline.
export function eventMatchesDate(e: CalendarEvent, date: string, tz: string) {
  return dayKey(e.start, tz) === date;
}

/** Resolve a wall-clock hour in the selected planning timezone, rejecting DST gaps. */
export function zonedMinute(date: string, hour: number, tz: string): number {
  const desired = Date.parse(`${date}T${String(hour).padStart(2, "0")}:00:00Z`);
  let candidate = desired;
  for (let i = 0; i < 3; i++) {
    const parts = new Intl.DateTimeFormat("en-CA", {
      timeZone: tz,
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      hourCycle: "h23",
    }).formatToParts(new Date(candidate));
    const get = (type: string) => parts.find((p) => p.type === type)!.value;
    const wall = Date.parse(
      `${get("year")}-${get("month")}-${get("day")}T${get("hour")}:${get("minute")}:00Z`,
    );
    const delta = desired - wall;
    if (delta === 0) return candidate / 60000;
    candidate += delta;
  }
  throw Error("该小时在所选时区不存在，请通过调整时间对话框选择。");
}
