import {
  AlertTriangle,
  ArrowRight,
  ArrowUpRight,
  CheckCircle2,
  GitBranch,
  Undo2,
} from "lucide-react";
import { clock, dayKey, duration } from "../../utils";

import Empty from "../../components/Empty";
import type { PageContext } from "../../shell/contracts";
export default function AdjustmentsPage(props: PageContext) {
  const {
    data,
    page,
    busy,
    today,
    date,
    tz,
    dayOffset,
    setDayOffset,
    setModal,
    setPage,
    query,
    setQuery,
    events,
    blocks,
    scheduledMinutes,
    openTasks,
    risks,
    active,
    next,
    name,
    changeAttention,
    run,
    action,
    setToast,
  } = props;
  return (
    <div className="adjustment-layout">
      <section className="card">
        <div className="card-heading">
          <div>
            <h2>最近一次规划</h2>
            <span>
              {data.plan
                ? `${clock(data.plan.createdAt, tz)} · ${data.plan.reason}`
                : "还没有生成计划"}
            </span>
          </div>
          <button
            className="button small"
            disabled={busy || !data.history.length}
            onClick={() => run("undo", null, "已结合最新执行事实恢复安排")}
          >
            <Undo2 size={15} /> 撤销安排
          </button>
        </div>
        {data.plan ? (
          <>
            <div className="impact-summary">
              <span>
                <b>{data.plan.proposal.changes.length}</b> 个任务调整
              </span>
              <span>
                <b>{data.plan.impact.affected.length}</b> 个任务受影响
              </span>
              <span>
                <b>{risks.length}</b> 项尚未排下
              </span>
            </div>
            {data.plan.impact.edges.length > 0 && (
              <div className="impact-edges">
                {data.plan.impact.edges.map((e, i) => (
                  <div key={i}>
                    <GitBranch size={16} />
                    <span>{name(e.from)}</span>
                    <ArrowRight size={15} />
                    <span>{name(e.to)}</span>
                    <small>
                      {e.kind === "dependency" ? "前置依赖" : "时间竞争"}
                    </small>
                  </div>
                ))}
              </div>
            )}
            {data.plan.proposal.changes.map((c) => (
              <div className="change-row" key={c.taskId}>
                <h3>{name(c.taskId)}</h3>
                <div>
                  <span>
                    {c.before.length
                      ? c.before
                          .map(
                            (b) =>
                              `${dayKey(b.start, tz).slice(5)} ${clock(b.start, tz)}`,
                          )
                          .join("、")
                      : "未安排"}
                  </span>
                  <ArrowRight size={15} />
                  <b>
                    {c.after.length
                      ? c.after
                          .map(
                            (b) =>
                              `${dayKey(b.start, tz).slice(5)} ${clock(b.start, tz)}`,
                          )
                          .join("、")
                      : "待重新安排"}
                  </b>
                </div>
                <p>{c.reason}</p>
              </div>
            ))}
            {!data.plan.proposal.changes.length && (
              <Empty
                title="安排保持稳定"
                body="没有必要的变化，就不打乱你的节奏。"
              />
            )}
          </>
        ) : (
          <Empty
            title="还没有调整记录"
            body="添加任务并生成计划后，可以在这里查看变化。"
          />
        )}
      </section>
      <aside>
        <section className="card">
          <div className="card-heading">
            <h2>需要你留意</h2>
            <AlertTriangle size={18} />
          </div>
          {risks.length === 0 && !data.plan?.proposal.risks.length ? (
            <div className="good-message">
              <CheckCircle2 size={24} />
              <h3>暂时没有待解决风险</h3>
              <p>截止风险和连续延期将在这里出现。</p>
            </div>
          ) : (
            <div className="risk-list">
              {risks.map((r) => (
                <div key={r.taskId}>
                  <b>{name(r.taskId)}</b>
                  <p>剩余 {duration(r.minutes)} 未安排</p>
                  <small>{r.reason}</small>
                  <button
                    className="text-button"
                    onClick={() => {
                      const t = data.tasks.find((t) => t.id === r.taskId);
                      if (t) setModal({ type: "task", task: t });
                    }}
                  >
                    检查任务与截止时间 <ArrowUpRight size={14} />
                  </button>
                </div>
              ))}
              {data.plan?.proposal.risks.map((r, i) => (
                <p className="warning-text" key={i}>
                  {r}
                </p>
              ))}
            </div>
          )}
        </section>
        <section className="card history-card">
          <h3>计划版本</h3>
          {[...(data.plan ? [data.plan] : []), ...data.history]
            .slice(0, 8)
            .map((p, i) => (
              <div key={p.id}>
                <span className="history-dot" />
                <div>
                  <b>
                    {i === 0
                      ? "当前版本"
                      : `历史版本 ${data.history.length - i + 1}`}
                  </b>
                  <small>
                    {dayKey(p.createdAt, tz).slice(5)} {clock(p.createdAt, tz)}{" "}
                    · {p.proposal.changes.length} 项变化
                  </small>
                </div>
              </div>
            ))}
        </section>
      </aside>
    </div>
  );
}
