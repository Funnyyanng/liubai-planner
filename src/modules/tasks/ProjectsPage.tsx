import { Check, MoreHorizontal, Plus, Search } from "lucide-react";
import { dayKey, duration } from "../../utils";

import Empty from "../../components/Empty";
import type { PageContext } from "../../shell/contracts";
export default function ProjectsPage(props: PageContext) {
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
    <>
      <div className="toolbar">
        <div className="search">
          <Search size={16} />
          <input
            placeholder="搜索任务或项目…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>
        <span>
          {openTasks.length} 待完成 / {data.tasks.length} 总任务
        </span>
      </div>
      {data.tasks.length === 0 ? (
        <Empty
          title="把大目标变成下一步"
          body="创建任务，或让 AI 帮你拆解一个项目。"
          action={
            <button
              className="button primary"
              onClick={() => setModal({ type: "task" })}
            >
              创建任务
            </button>
          }
        />
      ) : (
        Object.entries(
          Object.groupBy(
            data.tasks.filter((t) => (t.title + t.project).includes(query)),
            (t) => t.project || "个人事务",
          ),
        ).map(([project, ts]) => (
          <section className="card project-card" key={project}>
            <div className="card-heading">
              <div>
                <h2>
                  <span className="project-dot" />
                  {project}
                </h2>
                <span>
                  {ts!.filter((t) => t.completed).length} / {ts!.length} 已完成
                  · 剩余{" "}
                  {duration(
                    ts!
                      .filter((t) => !t.completed)
                      .reduce((n, t) => n + t.remaining, 0),
                  )}
                </span>
              </div>
              <button
                className="icon-button"
                onClick={() => setModal({ type: "task" })}
              >
                <Plus size={18} />
              </button>
            </div>
            {ts!.map((t) => (
              <div
                className={`task-row ${t.completed ? "done" : ""}`}
                key={t.id}
              >
                <button
                  className="task-check"
                  aria-label={`更新${t.title}进度`}
                  onClick={() => setModal({ type: "feedback", task: t })}
                >
                  {t.completed && <Check size={14} />}
                </button>
                <div className="task-main">
                  <button onClick={() => setModal({ type: "task", task: t })}>
                    {t.title}
                  </button>
                  <small>
                    {t.completionCriteria || "尚未设置完成标准"}
                    {t.dependencies.length > 0 &&
                      ` · 前置：${t.dependencies.map(name).join("、")}`}
                  </small>
                </div>
                {t.blocked && <span className="tag amber">已阻塞</span>}
                {t.deferrals >= 2 && (
                  <span className="tag amber">复查估时</span>
                )}
                <span className={`priority p${t.priority}`}>P{t.priority}</span>
                <span className="task-duration">{duration(t.remaining)}</span>
                <span className="task-due">
                  {dayKey(t.deadline, tz).slice(5)} 截止
                </span>
                <button
                  className="icon-button"
                  aria-label={`编辑${t.title}`}
                  onClick={() => setModal({ type: "task", task: t })}
                >
                  <MoreHorizontal size={17} />
                </button>
              </div>
            ))}
          </section>
        ))
      )}
    </>
  );
}
