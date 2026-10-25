import { BookOpen } from "lucide-react";
import { useState } from "react";
import type { CalendarEvent } from "../../types";
import { datetimeInput, inputMinute, minute, uid } from "../../utils";

export default function EventForm({
  event,
  busy,
  onSave,
}: {
  event?: CalendarEvent;
  busy: boolean;
  onSave: (e: CalendarEvent) => Promise<void>;
}) {
  const [e, set] = useState<CalendarEvent>(
    event ?? {
      id: uid(),
      title: "",
      start: minute() + 60,
      end: minute() + 120,
      location: "",
      equipment: ["电脑"],
      recurrence: null,
      source: "manual",
    },
  );
  return (
    <form
      className="form"
      onSubmit={(v) => {
        v.preventDefault();
        void onSave(e);
      }}
    >
      <label>
        日程名称
        <input
          required
          value={e.title}
          placeholder="例如：计算机网络"
          onChange={(v) => set({ ...e, title: v.target.value })}
        />
      </label>
      <div className="form-grid">
        <label>
          开始（本机时区）
          <input
            required
            type="datetime-local"
            value={datetimeInput(e.start)}
            onChange={(v) => set({ ...e, start: inputMinute(v.target.value) })}
          />
        </label>
        <label>
          结束（本机时区）
          <input
            required
            type="datetime-local"
            value={datetimeInput(e.end)}
            onChange={(v) => set({ ...e, end: inputMinute(v.target.value) })}
          />
        </label>
        <label>
          地点
          <input
            value={e.location}
            placeholder="教学楼 A203"
            onChange={(v) => set({ ...e, location: v.target.value })}
          />
        </label>
        <label>
          可用设备
          <input
            value={e.equipment.join(",")}
            onChange={(v) =>
              set({
                ...e,
                equipment: v.target.value
                  .split(/[,，]/)
                  .map((x) => x.trim())
                  .filter(Boolean),
              })
            }
          />
        </label>
      </div>
      <label className="checkbox">
        <input
          type="checkbox"
          checked={!!e.recurrence}
          onChange={(v) =>
            set({
              ...e,
              recurrence: v.target.checked
                ? {
                    until: e.start + 16 * 7 * 1440,
                    intervalWeeks: 1,
                    weekdays: [(new Date(e.start * 60000).getDay() + 6) % 7],
                    excludedDates: [],
                  }
                : null,
            })
          }
        />{" "}
        按学期重复
      </label>
      {e.recurrence && (
        <>
          <div className="form-grid">
            <label>
              重复间隔
              <select
                value={e.recurrence.intervalWeeks}
                onChange={(v) =>
                  set({
                    ...e,
                    recurrence: {
                      ...e.recurrence!,
                      intervalWeeks: +v.target.value,
                    },
                  })
                }
              >
                <option value={1}>每周</option>
                <option value={2}>隔周（以首次日期为起点）</option>
              </select>
            </label>
            <label>
              学期结束
              <input
                type="datetime-local"
                value={datetimeInput(e.recurrence.until)}
                onChange={(v) =>
                  set({
                    ...e,
                    recurrence: {
                      ...e.recurrence!,
                      until: inputMinute(v.target.value),
                    },
                  })
                }
              />
            </label>
          </div>
          <div className="weekday-checks">
            {["一", "二", "三", "四", "五", "六", "日"].map((d, i) => (
              <label key={d}>
                <input
                  type="checkbox"
                  checked={e.recurrence!.weekdays.includes(i)}
                  onChange={(v) =>
                    set({
                      ...e,
                      recurrence: {
                        ...e.recurrence!,
                        weekdays: v.target.checked
                          ? [...e.recurrence!.weekdays, i]
                          : e.recurrence!.weekdays.filter((x) => x !== i),
                      },
                    })
                  }
                />
                <span>周{d}</span>
              </label>
            ))}
          </div>
          <label>
            停课日期（逗号分隔，YYYY-MM-DD）
            <input
              placeholder="2026-10-01,2026-10-02"
              value={e.recurrence.excludedDates.join(",")}
              onChange={(v) =>
                set({
                  ...e,
                  recurrence: {
                    ...e.recurrence!,
                    excludedDates: v.target.value
                      .split(/[,，]/)
                      .map((x) => x.trim())
                      .filter(Boolean),
                  },
                })
              }
            />
          </label>
        </>
      )}
      <div className="form-note">
        <BookOpen size={16} />{" "}
        新日程默认专注占用。保存后，在日历中调整单次注意力模式。
      </div>
      <button className="button primary full" type="submit" disabled={busy}>
        保存日程
      </button>
    </form>
  );
}
