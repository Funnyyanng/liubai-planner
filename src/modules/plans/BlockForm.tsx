import { useState } from "react";
import type { TimeBlock } from "../../types";
import { datetimeInput, inputMinute } from "../../utils";

export default function BlockForm({
  block,
  title,
  busy,
  onSave,
}: {
  block: TimeBlock;
  title: string;
  busy: boolean;
  onSave: (b: TimeBlock) => Promise<void>;
}) {
  const [b, set] = useState({ ...block, locked: true });
  return (
    <form
      className="form"
      onSubmit={(e) => {
        e.preventDefault();
        void onSave(b);
      }}
    >
      <h3>{title}</h3>
      <label>
        开始时间（本机时区）
        <input
          type="datetime-local"
          required
          value={datetimeInput(b.start)}
          onChange={(e) => {
            const start = inputMinute(e.target.value);
            set({ ...b, start, end: start + b.end - b.start });
          }}
        />
      </label>
      <label>
        结束时间（本机时区）
        <input
          type="datetime-local"
          required
          value={datetimeInput(b.end)}
          onChange={(e) => set({ ...b, end: inputMinute(e.target.value) })}
        />
      </label>
      <label className="checkbox">
        <input
          type="checkbox"
          checked={b.locked}
          onChange={(e) => set({ ...b, locked: e.target.checked })}
        />{" "}
        锁定这段安排，禁止自动移动
      </label>
      <p className="form-note">
        保存前会检查课程、生活保障、任务依赖和工作量。
      </p>
      <button type="submit" className="button primary full" disabled={busy}>
        保存时间块
      </button>
    </form>
  );
}
