import { invoke } from "@tauri-apps/api/core";
import { demoState } from "./demo";
import type { AiDraft, AppState, CalendarEvent } from "./types";
export const isDesktop = "__TAURI_INTERNALS__" in window;
export async function state() {
  return isDesktop ? invoke<AppState>("get_state") : structuredClone(demoState);
}
export async function dispatch(
  name: string,
  payload: unknown,
  version: number,
  commandId: string,
) {
  if (!isDesktop)
    throw Error("这是只读界面预览。请打开「留白」桌面应用进行操作。");
  return invoke<AppState>("dispatch", { name, payload, version, commandId });
}
export async function wake() {
  return isDesktop ? invoke<AppState>("wake") : structuredClone(demoState);
}
export async function draft(
  prompt: string,
  taskIds: string[],
  eventIds: string[],
) {
  if (!isDesktop) throw Error("AI 功能需要在桌面应用配置。");
  return invoke<AiDraft>("generate_draft", { prompt, taskIds, eventIds });
}
export async function occurrences(from: number, until: number) {
  return isDesktop
    ? invoke<CalendarEvent[]>("calendar_occurrences", { from, until })
    : demoState.calendar;
}
export async function desktop<T>(
  name: string,
  args: Record<string, unknown> = {},
) {
  if (!isDesktop) throw Error("此操作需要桌面应用。");
  return invoke<T>(name, args);
}
