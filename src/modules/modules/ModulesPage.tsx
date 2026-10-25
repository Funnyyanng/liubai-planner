import { Blocks } from "lucide-react";
import { moduleNames } from "../../utils";

import type { PageContext } from "../../shell/contracts";
export default function ModulesPage(props: PageContext) {
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
      <div className="framework-banner">
        <div className="framework-icon">
          <Blocks size={30} />
        </div>
        <div>
          <h2>一个基础框架，独立的能力模块</h2>
          <p>契约 v1 · 编译时注册 · 本地事务 · 版本化计划 · 持久化事件</p>
        </div>
        <span className="tag green">
          {data.modules.filter((m) => m.status === "ready").length} /{" "}
          {data.modules.length} 就绪
        </span>
      </div>
      <div className="module-grid">
        {data.modules.map((m) => (
          <section className="card module-card" key={m.id}>
            <div>
              <span className="module-icon">
                <Blocks size={20} />
              </span>
              <span className={`module-status ${m.status}`}>
                <i />
                {m.status === "ready"
                  ? "已就绪"
                  : m.status === "disabled"
                    ? "未启用"
                    : "需要处理"}
              </span>
            </div>
            <h3>{moduleNames[m.id]}</h3>
            <p>{m.detail}</p>
            <footer>
              <code>{m.id}</code>
              <span>v0.1.0</span>
            </footer>
          </section>
        ))}
      </div>
    </>
  );
}
