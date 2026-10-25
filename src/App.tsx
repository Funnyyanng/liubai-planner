import { setClock } from "./utils";
import {
  AlertTriangle,
  Bell,
  LoaderCircle,
  MoreHorizontal,
  Plus,
  RefreshCw,
  Settings,
  ShieldCheck,
  Sparkles,
  Upload,
  X,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import Empty from "./components/Empty";
import ModalShell from "./components/ModalShell";
import AiPanel from "./modules/ai/AiPanel";
import EventForm from "./modules/calendar/EventForm";
import FeedbackForm from "./modules/execution/FeedbackForm";
import BlockForm from "./modules/plans/BlockForm";
import TaskForm from "./modules/tasks/TaskForm";
import type { Modal, Page } from "./shell/contracts";
import { pages } from "./shell/registry";
import type { AppState, Attention, CalendarEvent } from "./types";
import { dayKey, errorText, minute, uid } from "./utils";
export default function App() {
  const [data, setData] = useState<AppState | null>(null),
    [page, setPage] = useState<Page>("now"),
    [modal, setModal] = useState<Modal>(null),
    [busy, setBusy] = useState(false),
    [toast, setToast] = useState(""),
    [fatal, setFatal] = useState(""),
    [today, setToday] = useState(minute()),
    [dayOffset, setDayOffset] = useState(0),
    [query, setQuery] = useState(""),
    [events, setEvents] = useState<CalendarEvent[]>([]);
  const latest = useRef(data);
  latest.current = data;
  const inflight = useRef(false);
  const interacting = useRef(false);
  interacting.current = !!modal || busy;
  const noticeIds = useRef(new Set<string>());
  const accept = (s: AppState) => {
    setClock(s.testClock ? s.currentTime : undefined);
    setData(s);
    setToday(s.currentTime ?? minute());
    setFatal("");
  };
  useEffect(() => {
    api
      .state()
      .then(accept)
      .catch((e) => setFatal(errorText(e)));
  }, []);
  useEffect(() => {
    const tick = setInterval(() => {
      if (!api.isDesktop) setToday(minute());
      if (api.isDesktop && !inflight.current && !interacting.current) {
        inflight.current = true;
        api
          .wake()
          .then(accept)
          .catch((e) => setToast(errorText(e)))
          .finally(() => {
            inflight.current = false;
          });
      }
    }, 60000);
    const focus = () => {
      if (api.isDesktop && !interacting.current && !inflight.current) {
        inflight.current = true;
        api
          .wake()
          .then(accept)
          .catch((e) => setToast(errorText(e)))
          .finally(() => (inflight.current = false));
      }
    };
    window.addEventListener("focus", focus);
    return () => {
      clearInterval(tick);
      window.removeEventListener("focus", focus);
    };
  }, []);
  useEffect(() => {
    if (!data) return;
    api
      .occurrences(today - 1440, today + 15 * 1440)
      .then(setEvents)
      .catch((e) => setToast(errorText(e)));
  }, [data?.version, today]);
  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(() => setToast(""), 6000);
    return () => clearTimeout(t);
  }, [toast]);
  useEffect(() => {
    if (!data) return;
    const fresh = data.notices.filter(
      (n) => !n.read && !noticeIds.current.has(n.id),
    );
    for (const n of data.notices) noticeIds.current.add(n.id);
    if (fresh.length && api.isDesktop && document.visibilityState === "hidden")
      void api
        .desktop("desktop_notice", {
          title: `留白 · ${fresh.length} 项更新`,
          body: fresh[0].title,
        })
        .catch(() => {});
  }, [data?.notices]);
  async function run(
    name: string,
    payload: unknown = null,
    message = "已更新",
  ) {
    if (inflight.current) return;
    inflight.current = true;
    setBusy(true);
    try {
      const s = await api.dispatch(
        name,
        payload,
        latest.current!.version,
        uid(),
      );
      accept(s);
      setToast(message);
      return s;
    } catch (e) {
      setToast(errorText(e));
      api
        .state()
        .then(accept)
        .catch(() => {});
      return undefined;
    } finally {
      inflight.current = false;
      setBusy(false);
    }
  }
  async function action(name: string, args: Record<string, unknown> = {}) {
    setBusy(true);
    try {
      const result = await api.desktop<unknown>(name, args);
      if (result && typeof result === "object" && "version" in result)
        accept(result as AppState);
      else if (result)
        setToast(typeof result === "string" ? result : "操作完成");
    } catch (e) {
      setToast(errorText(e));
    } finally {
      setBusy(false);
    }
  }
  if (!data)
    return (
      <div className="boot">
        <div className="brand-mark">留</div>
        <h1>留白</h1>
        {fatal ? (
          <>
            <p className="error">{fatal}</p>
            <p>已停止业务写入，原数据库不会被覆盖。</p>
            <button
              onClick={() =>
                api
                  .desktop<{ path: string; error: string }>("diagnostics")
                  .then((d) => setToast(`数据位置：${d.path}`))
                  .catch((e) => setToast(errorText(e)))
              }
            >
              查看数据位置
            </button>
            <button onClick={() => action("recover_database")}>
              从备份恢复
            </button>
          </>
        ) : (
          <>
            <LoaderCircle className="spin" />
            <p>正在打开你的规划空间</p>
          </>
        )}
        {toast && <p>{toast}</p>}
      </div>
    );
  const tz = data.rules.timezone;
  const date = dayKey(today + dayOffset * 1440, tz);
  const active = data.executions.find((e) => e.end === null);
  const openTasks = data.tasks.filter((t) => !t.completed);
  const blocks = (data.plan?.proposal.blocks ?? []).filter(
    (b) => dayKey(b.start, tz) === date,
  );
  const scheduledMinutes = blocks.reduce((n, b) => n + b.end - b.start, 0);
  const risks = data.plan?.proposal.unscheduled ?? [];
  const ready = openTasks.filter((t) => data.recommendations.includes(t.id));
  const next = active
    ? data.tasks.find((t) => t.id === active.taskId)
    : ready.sort(
        (a, b) => a.priority - b.priority || a.deadline - b.deadline,
      )[0];
  const name = (id: string) => data.tasks.find((t) => t.id === id)?.title ?? id;
  const header = pages.find((p) => p.id === page)!;
  const PageView = header.component;
  const changeAttention = async (e: CalendarEvent, a: Attention) => {
    await run(
      "setAttention",
      {
        id: `${e.id}:${dayKey(e.start, tz)}`,
        eventId: e.id,
        date: dayKey(e.start, tz),
        attention: a,
      },
      "本节课的可用性已更新，后续计划已检查",
    );
  };
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <a className="brand" onClick={() => setPage("now")}>
          <div className="brand-mark">留</div>
          <div>
            <strong>留白</strong>
            <span>给生活一点余地</span>
          </div>
        </a>
        <div className="workspace">
          <span className="avatar">我</span>
          <div>
            <b>我的规划空间</b>
            <small>
              <i /> 本地个人空间
            </small>
          </div>
          <MoreHorizontal size={16} />
        </div>
        <nav>
          {pages.map((p, i) => (
            <div key={p.id}>
              {i === 4 && <div className="nav-label">扩展与偏好</div>}
              <button
                className={`nav-item ${page === p.id ? "selected" : ""}`}
                onClick={() => {
                  setPage(p.id);
                  setQuery("");
                }}
              >
                <p.icon size={19} />
                <span>{p.title}</span>
                {p.id === "adjustments" && risks.length > 0 && (
                  <em>{risks.length}</em>
                )}
                {p.id === "training" && <small>扩展</small>}
              </button>
            </div>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="local-card">
            <ShieldCheck size={19} />
            <div>
              <b>你的数据，留在本地</b>
              <p>AI 只接收你选定的上下文</p>
            </div>
          </div>
          <button className="profile" onClick={() => setPage("settings")}>
            <span className="avatar pale">学</span>
            <div>
              <b>学生 · 个人空间</b>
              <small>专注，也享受生活</small>
            </div>
            <Settings size={16} />
          </button>
        </div>
      </aside>
      <div className="main-shell">
        <header className="topbar">
          <div className="breadcrumb">
            我的空间 <span>/</span> <b>{header.title}</b>
          </div>
          <div className="top-actions">
            <span className="local-indicator">
              <i />
              {api.isDesktop ? "本地已连接" : "只读界面预览"}
            </span>
            <button
              className="icon-button"
              aria-label="查看提醒"
              onClick={() => setModal({ type: "notices" })}
            >
              <Bell size={18} />
              {data.notices.some((n) => !n.read) && <i className="dot" />}
            </button>
            <div className="divider" />
            <button
              className="button small"
              onClick={() => setModal({ type: "ai" })}
            >
              <Sparkles size={16} /> AI 助手
            </button>
          </div>
        </header>
        {!api.isDesktop && (
          <div className="preview-banner">
            界面预览 ·
            以下为展示数据，真实任务、排程和本地存储请在桌面应用中使用。
          </div>
        )}
        <main>
          {data.testClock && (
            <div
              role="status"
              style={{
                padding: "10px 28px",
                background: "#fff0d0",
                color: "#765200",
              }}
            >
              测试时间：{dayKey(data.currentTime ?? today, data.rules.timezone)}{" "}
              · 系统时钟未改变
            </div>
          )}
          {data.diagnostic && (
            <div className="diagnostic-banner">
              <AlertTriangle size={18} />
              <div>
                <b>最新计划尚未生成，当前显示上次安排</b>
                <p>{data.diagnostic}</p>
              </div>
            </div>
          )}
          <div className="page-heading">
            <div>
              <div className="eyebrow">
                {page === "now"
                  ? new Intl.DateTimeFormat("zh-CN", {
                      timeZone: tz,
                      month: "long",
                      day: "numeric",
                      weekday: "long",
                    }).format(new Date(today * 60000))
                  : "留白 / " + header.title}
              </div>
              <h1>
                {page === "now" ? "为重要的事，留一点余地。" : header.title}
              </h1>
              <p>
                {
                  {
                    now: "计划跟着生活调整，而你只需要专注下一步。",
                    calendar: "课表是时间的边界，注意力决定更多可能。",
                    projects: "把大目标拆成可以开始的小事。",
                    adjustments: "看清每一次变化，以及它对后续安排的影响。",
                    canvas: "同步课程作业，让计划跟上学校安排。",
                    training: "通过独立模块，把训练与恢复融入一周。",
                    modules: "各司其职，通过统一契约协作。",
                    settings: "定义你的生活边界，让计划更适合你。",
                  }[page]
                }
              </p>
            </div>
            <div className="heading-actions">
              {page === "calendar" ? (
                <>
                  <button
                    className="button"
                    disabled={busy}
                    onClick={() =>
                      action("import_file", { version: data.version })
                    }
                  >
                    <Upload size={16} /> 导入课表
                  </button>
                  <button
                    className="button primary"
                    onClick={() => setModal({ type: "event" })}
                  >
                    <Plus size={17} /> 添加日程
                  </button>
                </>
              ) : (
                <>
                  <button
                    className="button"
                    disabled={busy}
                    onClick={() =>
                      run("replan", null, "已根据最新进度检查计划")
                    }
                  >
                    <RefreshCw size={16} className={busy ? "spin" : ""} />{" "}
                    重新规划
                  </button>
                  <button
                    className="button primary"
                    onClick={() => setModal({ type: "task" })}
                  >
                    <Plus size={17} /> 添加任务
                  </button>
                </>
              )}
            </div>
          </div>
          <PageView
            {...{
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
            }}
          />
          <footer className="page-footer">
            <span>留白 · 个人规划</span>
            <span>让计划适应生活</span>
            <span>{api.isDesktop ? "本地存储" : "界面预览"} · v0.1.0</span>
          </footer>
        </main>
      </div>
      {busy && (
        <div className="working">
          <LoaderCircle size={15} className="spin" /> 正在处理
        </div>
      )}
      {toast && (
        <div className="toast" role="status">
          <span>{toast}</span>
          <button onClick={() => setToast("")} aria-label="关闭提示">
            <X size={16} />
          </button>
        </div>
      )}
      {modal && (
        <ModalShell
          title={
            {
              task:
                modal.type === "task" && modal.task ? "编辑任务" : "添加任务",
              event:
                modal.type === "event" && modal.event ? "编辑日程" : "添加日程",
              feedback: "更新实际进度",
              block: "调整执行时间",
              ai: "AI 规划助手",
              notices: "提醒与反馈",
            }[modal.type]
          }
          onClose={() => {
            if (!busy) setModal(null);
          }}
          wide={modal.type === "ai"}
        >
          {modal.type === "task" && (
            <TaskForm
              key={modal.task?.id ?? "new"}
              task={modal.task}
              tasks={data.tasks}
              busy={busy}
              onSave={async (t) => {
                if (await run("saveTask", t, "任务已保存并检查后续计划"))
                  setModal(null);
              }}
            />
          )}
          {modal.type === "event" && (
            <EventForm
              event={modal.event}
              busy={busy}
              onSave={async (e) => {
                if (await run("saveEvent", e, "日程已保存")) setModal(null);
              }}
            />
          )}
          {modal.type === "feedback" && (
            <FeedbackForm
              task={modal.task}
              tasks={data.tasks}
              busy={busy}
              onSave={async (f) => {
                if (await run("feedback", f, "进度已更新，连锁影响已检查"))
                  setModal(null);
              }}
            />
          )}
          {modal.type === "block" && (
            <BlockForm
              block={modal.block}
              title={name(modal.block.taskId)}
              busy={busy}
              onSave={async (b) => {
                if (await run("moveBlock", b, "时间块已调整")) setModal(null);
              }}
            />
          )}
          {modal.type === "ai" && (
            <AiPanel
              data={data}
              onState={accept}
              onApply={async (d) => {
                if (await run("applyDraft", d, "草案已应用，计划已更新"))
                  setModal(null);
              }}
              onSettings={() => {
                setModal(null);
                setPage("settings");
              }}
            />
          )}
          {modal.type === "notices" && (
            <div className="notices">
              {data.notices.length === 0 ? (
                <Empty
                  title="没有新的提醒"
                  body="时间块结束后，我们会请你确认执行结果。"
                />
              ) : (
                data.notices.map((n) => (
                  <article key={n.id} className={n.read ? "read" : ""}>
                    <h3>{n.title}</h3>
                    <p>{n.body}</p>
                    <button
                      className="text-button"
                      disabled={n.read}
                      onClick={() => run("markRead", n.id, "已标记已读")}
                    >
                      {n.read ? "已读" : "标记已读"}
                    </button>
                  </article>
                ))
              )}
            </div>
          )}
        </ModalShell>
      )}
    </div>
  );
}
