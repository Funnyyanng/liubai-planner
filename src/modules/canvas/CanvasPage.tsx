import { useState } from "react";
import { desktop } from "../../api";
import type { CalendarEvent } from "../../types";
import type { PageContext } from "../../shell/contracts";
type Course = { id: number; name: string };
type Preview = {
  course: Course;
  assignments: {
    id: number;
    title: string;
    url: string;
    due: number | null;
    submitted: boolean;
  }[];
  events: CalendarEvent[];
  undated: { id: number; title: string; url: string }[];
  resources: { title: string; url: string }[];
  warnings: string[];
};
export default function CanvasPage({ data, action }: PageContext) {
  const [reports, setReports] = useState<Preview[]>([]);
  const [token, setToken] = useState("");
  const [courses, setCourses] = useState<Course[]>([]);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [selected, setSelected] = useState<number[]>([]);
  const [minutes, setMinutes] = useState(60);
  const [events, setEvents] = useState(true);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  async function perform(work: () => Promise<void>) {
    setBusy(true);
    setMessage("");
    try {
      await work();
    } catch (e) {
      setMessage(
        e instanceof Error
          ? e.message
          : typeof e === "object" && e && "message" in e
            ? String(e.message)
            : "连接失败，请重试",
      );
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="panel" style={{ padding: 28 }}>
      <h2>连接交大 Canvas</h2>
      <p>
        自动检查所有课程，仅显示待完成作业和未来 60 天内的日程。学校侧只读。
      </p>
      <div style={{ display: "flex", gap: 12, margin: "20px 0" }}>
        <input
          aria-label="Canvas 访问令牌"
          type="password"
          value={token}
          autoComplete="off"
          placeholder="访问令牌（仅保存至钥匙串）"
          onChange={(e) => setToken(e.target.value)}
        />
        <button
          disabled={busy}
          onClick={() =>
            perform(async () => {
              if (token) {
                await desktop("canvas_save_token", { token });
                setToken("");
              }
              const all = await desktop<Preview[]>("canvas_scan");
              setReports(all);
              const relevant = all.filter(
                (p) => p.assignments.length > 0 || p.events.length > 0,
              );
              setCourses(relevant.map((p) => p.course));
              setPreview(relevant[0] ?? null);
              setSelected(relevant[0]?.assignments.map((a) => a.id) ?? []);
              setMessage(
                `已检查 ${all.length} 门课程，保留 ${relevant.length} 门有未来待办或日程的课程。`,
              );
            })
          }
        >
          {busy ? "正在检查课程…" : "智能整理待办"}
        </button>
      </div>
      {message && <p role="status">{message}</p>}
      <p>
        参考时间：
        {new Date(
          (data.currentTime ?? Date.now() / 60000) * 60000,
        ).toLocaleString("zh-CN", { timeZone: data.rules.timezone })}
        {data.testClock ? "（测试）" : ""}
        。已提交和截止时间已过的作业默认隐藏，不删除本地记录。
      </p>
      {reports.some((p) =>
        p.warnings.some((w) => w.startsWith("未能检查")),
      ) && (
        <div role="alert">
          <h3>部分课程尚未检查成功</h3>
          {reports
            .filter((p) => p.warnings.some((w) => w.startsWith("未能检查")))
            .map((p) => (
              <p key={p.course.id}>
                {p.course.name}：{p.warnings.join("；")}
              </p>
            ))}
          <p>这些课程不能视为没有待办，请稍后重新整理。</p>
        </div>
      )}
      {reports.some((p) => p.undated.length > 0) && (
        <details>
          <summary>
            待确认：{reports.reduce((n, p) => n + p.undated.length, 0)}{" "}
            项未提交作业没有截止时间
          </summary>
          {reports.flatMap((p) =>
            p.undated.map((a) => (
              <p key={`${p.course.id}:${a.id}`}>
                {p.course.name} ·{" "}
                <a
                  href={a.url}
                  onClick={(e) => {
                    e.preventDefault();
                    void desktop("canvas_open_page", { url: a.url });
                  }}
                >
                  {a.title}
                </a>
              </p>
            )),
          )}
        </details>
      )}

      <label>
        课程{" "}
        <select
          disabled={busy}
          value={preview?.course.id ?? ""}
          onChange={(e) => {
            const courseId = Number(e.target.value);
            if (!courseId) return;
            void perform(async () => {
              setPreview(null);
              setSelected([]);
              const p = await desktop<Preview>("canvas_preview", { courseId });
              setPreview(p);
              setSelected(p.assignments.map((a) => a.id));
            });
          }}
        >
          <option value="">请选择课程</option>
          {courses.map((c) => (
            <option key={c.id} value={c.id}>
              {c.name}
            </option>
          ))}
        </select>
      </label>
      {preview && (
        <>
          <h3>{preview.course.name}</h3>
          <button
            disabled={busy}
            onClick={() =>
              perform(async () => {
                const p = await desktop<Preview>("canvas_preview", {
                  courseId: preview.course.id,
                });
                setPreview(p);
                setSelected(p.assignments.map((a) => a.id));
              })
            }
          >
            刷新当前课程
          </button>
          <p>
            已自动选中未来待完成作业，可取消不需要的项目。新任务估时由你设定；已有任务保留剩余用时、完成状态、依赖和本地标题，仅更新学校截止时间。
          </p>
          {preview.warnings.map((w, i) => (
            <p key={i}>{w}</p>
          ))}
          {preview.assignments.length === 0 && (
            <p>这门课程没有未来待完成作业。</p>
          )}
          {preview.assignments.map((a) => (
            <div
              key={a.id}
              style={{ padding: "12px 0", borderBottom: "1px solid #eee" }}
            >
              <label>
                <input
                  type="checkbox"
                  disabled={busy || a.due === null}
                  checked={selected.includes(a.id)}
                  onChange={(e) =>
                    setSelected(
                      e.target.checked
                        ? [...selected, a.id]
                        : selected.filter((i) => i !== a.id),
                    )
                  }
                />{" "}
                {a.title}
              </label>
              <p>
                {a.due === null
                  ? "未设置截止时间，请手动建任务"
                  : new Date(a.due * 60000).toLocaleString("zh-CN", {
                      timeZone: data.rules.timezone,
                    })}{" "}
                ·{" "}
                {a.submitted
                  ? "Canvas 已提交／已评阅，本地完成状态不自动改变"
                  : "尚无提交记录"}{" "}
                ·{" "}
                <a
                  onClick={(e) => {
                    e.preventDefault();
                    void perform(async () => {
                      await desktop("canvas_open_page", { url: a.url });
                    });
                  }}
                  href={a.url}
                  target="_blank"
                  rel="noreferrer"
                >
                  原作业
                </a>
              </p>
            </div>
          ))}
          <p>
            <label>
              新任务预计用时（分钟）{" "}
              <input
                aria-label="Canvas 新任务预计用时"
                type="number"
                min={5}
                max={10000}
                value={minutes}
                onChange={(e) => setMinutes(Number(e.target.value))}
              />
            </label>
          </p>
          <label>
            <input
              type="checkbox"
              checked={events}
              onChange={(e) => setEvents(e.target.checked)}
            />{" "}
            同步 {preview.events.length} 条日历事件（默认专注，可在日历调整）
          </label>
          <p>本次未出现的历史记录不会自动删除。同步失败不会删除本地数据。</p>
          <button
            disabled={
              busy || (!selected.length && (!events || !preview.events.length))
            }
            onClick={() =>
              perform(async () => {
                await action("canvas_import", {
                  request: {
                    courseId: preview.course.id,
                    assignmentIds: selected,
                    minutes,
                    importEvents: events,
                  },
                  preview,
                  version: data.version,
                  commandId: crypto.randomUUID(),
                });
              })
            }
          >
            应用选择并重新规划
          </button>
          <h3>课程资料（{preview.resources.length}）</h3>
          <p>点击“刷新当前课程”可按需读取课件链接。</p>
          {preview.resources.map((r, i) => (
            <p key={i}>
              <a
                onClick={(e) => {
                  e.preventDefault();
                  void perform(async () => {
                    await desktop("canvas_open_page", { url: r.url });
                  });
                }}
                href={r.url}
                target="_blank"
                rel="noreferrer"
              >
                {r.title}
              </a>
            </p>
          ))}
          <p>资料仅展示入口，尚未下载或发送给 AI。</p>
        </>
      )}
    </section>
  );
}
