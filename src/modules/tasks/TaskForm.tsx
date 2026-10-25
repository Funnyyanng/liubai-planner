import { ArrowRight, ShieldCheck, SlidersHorizontal } from "lucide-react";
import { useState } from "react";
import type { Task } from "../../types";
import {
  categoryNames,
  datetimeInput,
  inputMinute,
  minute,
  uid,
} from "../../utils";

export default function TaskForm({
  task,
  tasks,
  busy,
  onSave,
}: {
  task?: Task;
  tasks: Task[];
  busy: boolean;
  onSave: (t: Task) => Promise<void>;
}) {
  const [t, set] = useState<Task>(
    task ?? {
      id: uid(),
      title: "",
      project: "",
      category: "study",
      remaining: 60,
      estimate: 60,
      deadline: minute() + 3 * 1440,
      priority: 1,
      minChunk: 25,
      splittable: true,
      conditions: {
        focus: 2,
        interruptible: true,
        equipment: [],
        location: null,
      },
      dependencies: [],
      completed: false,
      blocked: false,
      completionCriteria: "",
      deferrals: 0,
      parentId: null,
    },
  );
  const patch = (p: Partial<Task>) => set({ ...t, ...p });
  return (
    <form
      className="form"
      onSubmit={(e) => {
        e.preventDefault();
        void onSave({ ...t, estimate: task ? t.estimate : t.remaining });
      }}
    >
      <label>
        任务名称
        <input
          autoFocus
          required
          placeholder="例如：完成实验数据分析"
          value={t.title}
          onChange={(e) => patch({ title: e.target.value })}
        />
      </label>
      <div className="form-grid">
        <label>
          所属项目
          <input
            placeholder="例如：课题研究"
            value={t.project}
            onChange={(e) => patch({ project: e.target.value })}
          />
        </label>
        <label>
          任务领域
          <select
            value={t.category}
            onChange={(e) => patch({ category: e.target.value })}
          >
            {Object.entries(categoryNames).map(([k, v]) => (
              <option key={k} value={k}>
                {v}
              </option>
            ))}
          </select>
        </label>
        <label>
          剩余工作量（分钟）
          <input
            type="number"
            min={t.completed ? 0 : 5}
            max={100000}
            required
            value={t.remaining}
            onChange={(e) => patch({ remaining: +e.target.value })}
          />
        </label>
        <label>
          截止时间（本机时区）
          <input
            required
            type="datetime-local"
            value={datetimeInput(t.deadline)}
            onChange={(e) => patch({ deadline: inputMinute(e.target.value) })}
          />
        </label>
        <label>
          优先级
          <select
            value={t.priority}
            onChange={(e) => patch({ priority: +e.target.value })}
          >
            {["P0 · 最高", "P1 · 重要", "P2 · 普通", "P3 · 灵活"].map(
              (v, i) => (
                <option value={i} key={i}>
                  {v}
                </option>
              ),
            )}
          </select>
        </label>
        <label>
          专注要求
          <select
            value={t.conditions.focus}
            onChange={(e) =>
              patch({ conditions: { ...t.conditions, focus: +e.target.value } })
            }
          >
            <option value={0}>几乎不需专注</option>
            <option value={1}>轻度专注</option>
            <option value={2}>中度专注</option>
            <option value={3}>深度专注</option>
          </select>
        </label>
      </div>
      <label>
        完成标准
        <textarea
          rows={2}
          placeholder="做到什么程度才算完成？"
          value={t.completionCriteria}
          onChange={(e) => patch({ completionCriteria: e.target.value })}
        />
      </label>
      <details>
        <summary>
          <SlidersHorizontal size={15} /> 执行条件与依赖
        </summary>
        <div className="form-grid">
          <label>
            最小连续时长（分钟）
            <input
              type="number"
              min={5}
              max={1440}
              value={t.minChunk}
              onChange={(e) => patch({ minChunk: +e.target.value })}
            />
          </label>
          <label>
            地点限制
            <input
              placeholder="留空表示不限"
              value={t.conditions.location ?? ""}
              onChange={(e) =>
                patch({
                  conditions: {
                    ...t.conditions,
                    location: e.target.value || null,
                  },
                })
              }
            />
          </label>
          <label>
            设备（逗号分隔）
            <input
              placeholder="电脑"
              value={t.conditions.equipment.join(",")}
              onChange={(e) =>
                patch({
                  conditions: {
                    ...t.conditions,
                    equipment: e.target.value
                      .split(/[,，]/)
                      .map((s) => s.trim())
                      .filter(Boolean),
                  },
                })
              }
            />
          </label>
          <label className="checkbox">
            <input
              type="checkbox"
              checked={t.splittable}
              onChange={(e) => patch({ splittable: e.target.checked })}
            />{" "}
            允许分多次完成
          </label>
          <label className="checkbox">
            <input
              type="checkbox"
              checked={t.conditions.interruptible}
              onChange={(e) =>
                patch({
                  conditions: {
                    ...t.conditions,
                    interruptible: e.target.checked,
                  },
                })
              }
            />{" "}
            可以被打断
          </label>
          <label className="checkbox">
            <input
              type="checkbox"
              checked={t.blocked}
              onChange={(e) => patch({ blocked: e.target.checked })}
            />{" "}
            目前遇到阻碍
          </label>
        </div>
        <label>必须先完成的任务</label>
        <div className="check-list">
          {tasks
            .filter((x) => x.id !== t.id)
            .map((x) => (
              <label className="checkbox" key={x.id}>
                <input
                  type="checkbox"
                  checked={t.dependencies.includes(x.id)}
                  onChange={(e) =>
                    patch({
                      dependencies: e.target.checked
                        ? [...t.dependencies, x.id]
                        : t.dependencies.filter((id) => id !== x.id),
                    })
                  }
                />
                {x.title}
              </label>
            ))}
          {tasks.length === 0 && <small>尚无其他任务</small>}
        </div>
      </details>
      <div className="form-note">
        <ShieldCheck size={15} />{" "}
        修改截止时间属于你的明确选择，系统不会自行更改。
      </div>
      <button className="button primary full" disabled={busy} type="submit">
        保存任务并检查计划 <ArrowRight size={16} />
      </button>
    </form>
  );
}
