import { Dumbbell } from "lucide-react";
import { useState } from "react";
import type { Task, TrainingConfig } from "../../types";
import { duration } from "../../utils";

import Empty from "../../components/Empty";
export default function Training({
  data,
  busy,
  onSave,
  tasks,
  onTask,
}: {
  data: TrainingConfig;
  busy: boolean;
  onSave: (v: TrainingConfig) => Promise<unknown>;
  tasks: Task[];
  onTask: (t: Task) => void;
}) {
  const [c, set] = useState(data);
  return (
    <div className="settings-layout">
      <section className="card settings-card">
        <div className="training-heading">
          <span>
            <Dumbbell size={28} />
          </span>
          <div>
            <h2>每周训练</h2>
            <p>独立扩展 · 使用统一任务与排程契约</p>
          </div>
        </div>
        <form
          className="form"
          onSubmit={(e) => {
            e.preventDefault();
            void onSave(c);
          }}
        >
          <label className="toggle-row">
            <span>
              <b>启用训练模块</b>
              <small>将训练需求加入滚动计划</small>
            </span>
            <input
              type="checkbox"
              checked={c.enabled}
              onChange={(e) => set({ ...c, enabled: e.target.checked })}
            />
          </label>
          <label>
            训练名称
            <input
              required
              value={c.title}
              onChange={(e) => set({ ...c, title: e.target.value })}
            />
          </label>
          <div className="form-grid">
            <label>
              每周次数
              <input
                required
                type="number"
                min={1}
                max={7}
                value={c.sessions}
                onChange={(e) => set({ ...c, sessions: +e.target.value })}
              />
            </label>
            <label>
              每次时长（分钟）
              <input
                required
                type="number"
                min={5}
                max={240}
                value={c.minutes}
                onChange={(e) => set({ ...c, minutes: +e.target.value })}
              />
            </label>
            <label>
              两次训练至少间隔（小时）
              <input
                required
                type="number"
                min={0}
                max={168}
                value={c.gapHours}
                onChange={(e) => set({ ...c, gapHours: +e.target.value })}
              />
            </label>
          </div>
          <button type="submit" disabled={busy} className="button primary">
            保存并更新训练安排
          </button>
        </form>
      </section>
      <section className="card settings-card">
        <h2>训练记录</h2>
        <p>记录完成状态后，后续安排会考虑恢复间隔。</p>
        {tasks.length ? (
          tasks.map((t) => (
            <div className="training-task" key={t.id}>
              <Dumbbell size={18} />
              <div>
                <b>{t.title}</b>
                <small>{t.completed ? "已完成" : duration(t.remaining)}</small>
              </div>
              <button className="button small" onClick={() => onTask(t)}>
                记录
              </button>
            </div>
          ))
        ) : (
          <Empty
            title="从合适的频率开始"
            body="启用模块后，训练任务会出现在这里。"
          />
        )}
      </section>
    </div>
  );
}
