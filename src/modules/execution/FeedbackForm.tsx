import {
  AlertTriangle,
  ArrowRight,
  CheckCircle2,
  Clock3,
  GitBranch,
  Pause,
} from "lucide-react";
import { useState } from "react";
import type { Task } from "../../types";
import { categoryNames, duration, minute } from "../../utils";

export default function FeedbackForm({
  task,
  tasks,
  busy,
  onSave,
}: {
  task: Task;
  tasks: Task[];
  busy: boolean;
  onSave: (f: unknown) => Promise<void>;
}) {
  const [outcome, setOutcome] = useState("unfinished"),
    [remaining, setRemaining] = useState(task.remaining),
    [actual, setActual] = useState(30),
    [note, setNote] = useState("");
  const followers = tasks.filter((t) => t.dependencies.includes(task.id));
  return (
    <form
      className="form"
      onSubmit={(e) => {
        e.preventDefault();
        void onSave({
          taskId: task.id,
          outcome,
          remaining: outcome === "completed" ? 0 : remaining,
          actualMinutes: outcome === "notStarted" ? 0 : actual,
          note,
          now: minute(),
        });
      }}
    >
      <div className="feedback-task">
        <span className="tag">{categoryNames[task.category]}</span>
        <h3>{task.title}</h3>
        <p>原剩余估时：{duration(task.remaining)}</p>
      </div>
      <div className="outcome-grid">
        {[
          ["completed", "已经完成", CheckCircle2],
          ["unfinished", "还没做完", Clock3],
          ["blocked", "遇到阻碍", AlertTriangle],
          ["notStarted", "还没有开始", Pause],
        ].map(([id, label, Icon]) => {
          const I = Icon as typeof Clock3;
          return (
            <button
              key={String(id)}
              type="button"
              className={outcome === id ? "chosen" : ""}
              onClick={() => setOutcome(String(id))}
            >
              <I size={18} />
              {String(label)}
            </button>
          );
        })}
      </div>
      <div className="form-grid">
        {outcome !== "notStarted" && (
          <label>
            本次实际用时（分钟）
            <input
              type="number"
              min={0}
              max={1440}
              required
              value={actual}
              onChange={(e) => setActual(+e.target.value)}
            />
          </label>
        )}
        {outcome !== "completed" && (
          <label>
            还需要多少分钟？
            <input
              type="number"
              min={5}
              max={100000}
              required
              value={remaining}
              onChange={(e) => setRemaining(+e.target.value)}
            />
          </label>
        )}
      </div>
      {outcome !== "completed" && (
        <div className="quick-values">
          {[15, 30, 60, 90, 120].map((n) => (
            <button
              type="button"
              className={remaining === n ? "chosen" : ""}
              key={n}
              onClick={() => setRemaining(n)}
            >
              还需 {n} 分钟
            </button>
          ))}
        </div>
      )}
      <label>
        备注 / 未完成原因
        <textarea
          placeholder="例如：实验结果异常，需要重新采集数据"
          value={note}
          onChange={(e) => setNote(e.target.value)}
        />
      </label>
      {followers.length > 0 && (
        <div className="impact-note">
          <GitBranch size={18} />
          <div>
            <b>将同时检查后续影响</b>
            <p>{followers.map((t) => t.title).join(" → ")}</p>
            <small>系统还会检查被挤占时间的其他任务。</small>
          </div>
        </div>
      )}
      <button type="submit" disabled={busy} className="button primary full">
        记录进度并检查连锁影响 <ArrowRight size={16} />
      </button>
    </form>
  );
}
