import {
  AlertTriangle,
  ArrowRight,
  ArrowUpRight,
  CheckCheck,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  Clock3,
  GitBranch,
  Leaf,
  Link2,
  LockKeyhole,
  MoreHorizontal,
  Pause,
  Play,
  Plus,
  ShieldCheck,
  Sparkles,
  Unlock,
} from "lucide-react";
import type { Attention } from "../../types";
import { categoryNames, clock, dayKey, duration } from "../../utils";

import Stat from "../../components/Stat";
import type { PageContext } from "../../shell/contracts";
import { attentionNames } from "../../shell/registry";
import { zonedMinute } from "../../utils";
export default function PlanningPage(props: PageContext) {
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
      <div className="stats">
        <Stat
          label="今天的安排"
          value={duration(scheduledMinutes)}
          detail={`${blocks.length} 个执行时间块`}
          icon={<Clock3 />}
        />
        <Stat
          label="待推进任务"
          value={`${openTasks.length}`}
          detail={`${data.tasks.filter((t) => t.completed).length} 项已完成`}
          icon={<CheckCheck />}
        />
        <Stat
          label="规划缓冲"
          value={`${data.rules.bufferPercent}%`}
          detail="为变化预留空间"
          icon={<Leaf />}
        />
        <Stat
          label="需要留意"
          value={`${risks.length}`}
          detail={risks.length ? "存在未安排的工作" : "当前计划没有容量缺口"}
          icon={<GitBranch />}
          warn={risks.length > 0}
        />
      </div>
      <div className="dashboard-grid">
        <section className="card schedule-card">
          <div className="card-heading">
            <div>
              <h2>{page === "calendar" ? "日程与执行安排" : "一天的节奏"}</h2>
              <span>
                {date} · {tz}
              </span>
            </div>
            <div className="day-controls">
              <button
                aria-label="前一天"
                onClick={() => setDayOffset((d) => d - 1)}
              >
                <ChevronLeft size={16} />
              </button>
              <button onClick={() => setDayOffset(0)}>
                {dayOffset === 0
                  ? "今天"
                  : dayOffset === 1
                    ? "明天"
                    : date.slice(5)}
              </button>
              <button
                aria-label="后一天"
                onClick={() => setDayOffset((d) => d + 1)}
              >
                <ChevronRight size={16} />
              </button>
            </div>
          </div>
          <div className="schedule-legend">
            <span>
              <i className="purple" />
              课题 / 学习
            </span>
            <span>
              <i className="blue" />
              固定日程
            </span>
            <span>
              <i className="green" />
              休息 / 生活
            </span>
            <span className="legend-end">拖动任务到小时可调整</span>
          </div>
          <div className="timeline">
            {Array.from(
              { length: data.rules.dayEnd - data.rules.dayStart },
              (_, i) => i + data.rules.dayStart,
            ).map((hour) => {
              const bs = blocks.filter(
                (b) => Number(clock(b.start, tz).split(":")[0]) === hour,
              );
              const es = events.filter(
                (e) =>
                  dayKey(e.start, tz) === date &&
                  Number(clock(e.start, tz).split(":")[0]) === hour,
              );
              const weekday = (new Date(date + "T12:00:00").getDay() + 6) % 7;
              const protection = data.rules.protected.find(
                (p) => p.startHour === hour && p.weekdays.includes(weekday),
              );
              return (
                <div
                  className="timeline-row"
                  key={hour}
                  onDragOver={(e) => e.preventDefault()}
                  onDrop={(e) => {
                    const block = blocks.find(
                      (b) => b.id === e.dataTransfer.getData("text/plain"),
                    );
                    if (block) {
                      let start: number;
                      try {
                        start = zonedMinute(date, hour, tz);
                      } catch (e) {
                        setToast(String(e));
                        return;
                      }
                      void run(
                        "moveBlock",
                        {
                          ...block,
                          start,
                          end: start + block.end - block.start,
                          locked: true,
                        },
                        "已移动并锁定时间块",
                      );
                    }
                  }}
                >
                  <div className="time-label">
                    {String(hour).padStart(2, "0")}:00
                  </div>
                  <div className="timeline-content">
                    {es.map((e) => {
                      const mode =
                        data.contexts.find(
                          (c) => c.eventId === e.id && c.date === date,
                        )?.attention ??
                        data.contexts.find(
                          (c) => c.eventId === e.id && c.date === null,
                        )?.attention ??
                        "focused";
                      return (
                        <div key={e.id + e.start} className="event-block">
                          <div className="block-top">
                            <span className="tag blue">课程 / 日程</span>
                            <span>
                              {clock(e.start, tz)} — {clock(e.end, tz)}
                            </span>
                            <button
                              className="icon-button"
                              aria-label={`编辑${e.title}`}
                              onClick={() =>
                                setModal({
                                  type: "event",
                                  event:
                                    data.calendar.find((x) => x.id === e.id) ??
                                    e,
                                })
                              }
                            >
                              <MoreHorizontal size={15} />
                            </button>
                          </div>
                          <b>{e.title}</b>
                          <div className="event-footer">
                            <small>{e.location || "地点未设置"}</small>
                            <select
                              aria-label={`${e.title}注意力模式`}
                              value={mode}
                              onChange={(v) =>
                                changeAttention(e, v.target.value as Attention)
                              }
                            >
                              {Object.entries(attentionNames).map(([k, v]) => (
                                <option key={k} value={k}>
                                  {v}
                                </option>
                              ))}
                            </select>
                          </div>
                        </div>
                      );
                    })}
                    {bs.map((b) => {
                      const t = data.tasks.find((t) => t.id === b.taskId);
                      if (!t) return null;
                      return (
                        <div
                          className={`task-block ${t.category}`}
                          key={b.id}
                          draggable={!b.active}
                          onDragStart={(e) =>
                            e.dataTransfer.setData("text/plain", b.id)
                          }
                        >
                          <div className="block-top">
                            <span className="tag">
                              {categoryNames[t.category] ?? "任务"}
                            </span>
                            <span>
                              {clock(b.start, tz)} — {clock(b.end, tz)}
                            </span>
                            <button
                              className="icon-button"
                              aria-label={
                                b.locked ? "解锁时间块" : "锁定时间块"
                              }
                              onClick={() =>
                                run(
                                  "moveBlock",
                                  { ...b, locked: !b.locked },
                                  b.locked ? "已解锁" : "已锁定",
                                )
                              }
                            >
                              {b.locked ? (
                                <LockKeyhole size={13} />
                              ) : (
                                <Unlock size={13} />
                              )}
                            </button>
                          </div>
                          <button
                            className="block-title"
                            onClick={() =>
                              setModal({ type: "feedback", task: t })
                            }
                          >
                            {t.title}
                          </button>
                          <div className="block-foot">
                            <span>
                              <Clock3 size={12} /> {duration(b.end - b.start)}
                            </span>
                            {t.dependencies.length > 0 && (
                              <span>
                                <Link2 size={12} /> 有前置任务
                              </span>
                            )}
                            <button
                              onClick={() =>
                                setModal({ type: "block", block: b })
                              }
                            >
                              调整时间 <ArrowUpRight size={13} />
                            </button>
                          </div>
                        </div>
                      );
                    })}
                    {protection && (
                      <div className="rest-block">
                        <Leaf size={16} />
                        <div>
                          <b>{protection.title}</b>
                          <small>
                            {protection.startHour}:00 — {protection.endHour}:00
                            · 生活保障
                          </small>
                        </div>
                      </div>
                    )}
                    {!bs.length && !es.length && !protection && (
                      <div className="empty-hour">
                        <span>
                          {events.some(
                            (e) =>
                              dayKey(e.start, tz) === date &&
                              Number(clock(e.start, tz).split(":")[0]) < hour &&
                              clock(e.end, tz) >
                                `${String(hour).padStart(2, "0")}:00`,
                          ) ||
                          blocks.some(
                            (b) =>
                              Number(clock(b.start, tz).split(":")[0]) < hour &&
                              clock(b.end, tz) >
                                `${String(hour).padStart(2, "0")}:00`,
                          )
                            ? "前一时段的安排仍在进行"
                            : "可安排 / 保留空白"}
                        </span>
                      </div>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
          <footer className="schedule-footer">
            <Leaf size={14} /> 没有排满的时间，也属于计划的一部分。
          </footer>
        </section>
        <aside className="right-column">
          <section className="focus-card">
            <div className="focus-label">
              <span>
                <i />
                {active ? "正在专注" : "你的下一步"}
              </span>
              <Sparkles size={16} />
            </div>
            {next ? (
              <>
                <span className="focus-project">
                  {next.project || "个人任务"}
                </span>
                <h2>{next.title}</h2>
                <p>{next.completionCriteria || "从一个具体的小步骤开始。"}</p>
                <div className="focus-meta">
                  <span>
                    <Clock3 size={15} />
                    {duration(next.remaining)}
                  </span>
                  <span>优先级 P{next.priority}</span>
                </div>
                {active ? (
                  <>
                    <div className="timer">
                      {duration(Math.max(0, today - active.start))}
                      <small>已经专注</small>
                    </div>
                    <button
                      className="focus-button"
                      disabled={busy}
                      onClick={() =>
                        run("pause", null, "已暂停，请按实际情况更新剩余时间")
                      }
                    >
                      <Pause size={16} /> 暂停一下
                    </button>
                  </>
                ) : (
                  <button
                    className="focus-button"
                    disabled={busy}
                    onClick={() => run("start", next.id, "已开始记录执行时间")}
                  >
                    <Play size={16} /> 开始专注 <ArrowRight size={16} />
                  </button>
                )}
                <button
                  className="focus-secondary"
                  onClick={() => setModal({ type: "feedback", task: next })}
                >
                  更新进度 / 没有做完 <ArrowUpRight size={14} />
                </button>
              </>
            ) : (
              <>
                <h2>
                  {openTasks.length ? (
                    <>现在，先留一点空白。</>
                  ) : (
                    <>
                      留一点空白，
                      <br />
                      从容开始。
                    </>
                  )}
                </h2>
                <p>
                  {openTasks.length
                    ? "当前条件下没有适合开始的任务。可以查看后续安排，或更新情境。"
                    : "添加你的第一个任务，或体验一周示例。"}
                </p>
                <button
                  className="focus-button"
                  onClick={() => setModal({ type: "task" })}
                >
                  <Plus size={16} />{" "}
                  {openTasks.length ? "添加任务" : "添加第一个任务"}
                </button>
                {data.tasks.length === 0 && (
                  <button
                    className="focus-secondary"
                    disabled={busy}
                    onClick={() =>
                      run("seedDemo", null, "已加入体验任务与课程")
                    }
                  >
                    载入体验数据 <ArrowUpRight size={14} />
                  </button>
                )}
              </>
            )}
          </section>
          <section className="card health-card">
            <div className="mini-title">
              <GitBranch size={17} />
              <h3>计划会跟着你调整</h3>
            </div>
            <p>
              任务超时后，先看依赖，再看时间。保持能保留的安排，指出需要取舍的地方。
            </p>
            <div className={`health-status ${risks.length ? "warning" : ""}`}>
              {risks.length ? (
                <AlertTriangle size={17} />
              ) : (
                <CheckCircle2 size={17} />
              )}
              <span>
                {risks.length
                  ? `${risks.length} 项任务还需要安排`
                  : "当前没有未解决的容量缺口"}
              </span>
            </div>
            <button
              className="text-button"
              onClick={() => setPage("adjustments")}
            >
              查看调整与影响 <ArrowRight size={15} />
            </button>
          </section>
          <section className="card balance-card">
            <h3>生活，也在计划里</h3>
            {data.rules.protected.map((p) => (
              <div className="balance-row" key={p.id}>
                <span className="small-icon">
                  <Leaf size={15} />
                </span>
                <div>
                  <b>{p.title}</b>
                  <small>
                    {p.startHour}:00 — {p.endHour}:00
                  </small>
                </div>
                <ShieldCheck size={15} />
              </div>
            ))}
            <button className="text-button" onClick={() => setPage("settings")}>
              调整生活规则 <ArrowRight size={15} />
            </button>
          </section>
        </aside>
      </div>
    </>
  );
}
