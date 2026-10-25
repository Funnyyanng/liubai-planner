import {
  ArrowRight,
  Check,
  GitBranch,
  LoaderCircle,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { useState, type FormEvent } from "react";
import * as api from "../../api";
import type { AiDraft, AppState } from "../../types";
import { duration, errorText } from "../../utils";

import Empty from "../../components/Empty";
import { attentionNames } from "../../shell/registry";
export default function AiPanel({
  data,
  onState,
  onApply,
  onSettings,
}: {
  data: AppState;
  onState: (s: AppState) => void;
  onApply: (d: AiDraft) => Promise<void>;
  onSettings: () => void;
}) {
  const [prompt, setPrompt] = useState(""),
    [taskIds, setTasks] = useState<string[]>([]),
    [eventIds, setEvents] = useState<string[]>([]),
    [draft, setDraft] = useState<AiDraft | null>(null),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  async function generate(e: FormEvent) {
    e.preventDefault();
    setError("");
    setBusy(true);
    try {
      setDraft(await api.draft(prompt, taskIds, eventIds));
      onState(await api.state());
    } catch (e) {
      setError(errorText(e));
      api
        .state()
        .then(onState)
        .catch(() => {});
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="ai-panel">
      <div className="ai-intro">
        <Sparkles size={24} />
        <div>
          <h3>说说你的目标，或者刚发生的变化。</h3>
          <p>AI 提出草案，本地引擎检查约束，重要决定留给你。</p>
        </div>
      </div>
      {!data.ai.enabled ? (
        <Empty
          title="连接你自己的 AI"
          body="在设置中填写模型 API 地址和密钥。任务数据保存在本地。"
          action={
            <button className="button primary" onClick={onSettings}>
              配置模型接口 <ArrowRight size={15} />
            </button>
          }
        />
      ) : (
        <form className="form" onSubmit={generate}>
          <label>
            规划需求
            <textarea
              rows={4}
              required
              maxLength={2000}
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder="例如：我需要在下周五交一份实验报告，请拆解为具体步骤。或者：这节课可以做别的事情，但可能被打断。"
            />
          </label>
          <details>
            <summary>
              <ShieldCheck size={15} /> 选择随请求发送的上下文（默认不发送）
            </summary>
            <div className="check-list">
              {data.tasks
                .filter((t) => !t.completed)
                .map((t) => (
                  <label key={t.id} className="checkbox">
                    <input
                      type="checkbox"
                      checked={taskIds.includes(t.id)}
                      onChange={(e) =>
                        setTasks(
                          e.target.checked
                            ? [...taskIds, t.id]
                            : taskIds.filter((x) => x !== t.id),
                        )
                      }
                    />
                    {t.title}
                  </label>
                ))}
              {data.calendar.map((e) => (
                <label key={e.id} className="checkbox">
                  <input
                    type="checkbox"
                    checked={eventIds.includes(e.id)}
                    onChange={(v) =>
                      setEvents(
                        v.target.checked
                          ? [...eventIds, e.id]
                          : eventIds.filter((x) => x !== e.id),
                      )
                    }
                  />
                  {e.title}（日历）
                </label>
              ))}
            </div>
          </details>
          <button className="button primary full" disabled={busy} type="submit">
            {busy ? (
              <LoaderCircle className="spin" size={17} />
            ) : (
              <Sparkles size={17} />
            )}{" "}
            {busy ? "正在生成草案…" : "生成可检查的草案"}
          </button>
          {error && <p className="error">{error}</p>}
        </form>
      )}
      {draft && (
        <div className="draft">
          <span className="tag">待你确认 · 尚未修改任务</span>
          <h3>{draft.summary}</h3>
          {draft.tasks.map((t, i) => (
            <article key={t.id}>
              <span>{i + 1}</span>
              <div>
                <b>{t.title}</b>
                <small>
                  {duration(t.remaining)} · {t.completionCriteria}
                </small>
              </div>
            </article>
          ))}
          {draft.actions.map((a, i) => (
            <article key={i}>
              <GitBranch size={18} />
              <div>
                {a.type === "remaining"
                  ? `更新剩余时间：${a.remaining} 分钟`
                  : `调整课程模式：${attentionNames[a.attention]}`}
              </div>
            </article>
          ))}
          <button
            className="button primary full"
            onClick={() => onApply(draft)}
          >
            采用草案并检查计划 <Check size={16} />
          </button>
        </div>
      )}
    </div>
  );
}
