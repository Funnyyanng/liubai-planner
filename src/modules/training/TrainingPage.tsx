import type { PageContext } from "../../shell/contracts";
import Training from "../training/Training";
export default function TrainingPage(props: PageContext) {
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
    <Training
      data={data.training}
      busy={busy}
      onSave={(v) => run("configureTraining", v, "训练模块配置已保存")}
      tasks={data.tasks.filter((t) => t.id.startsWith("training:"))}
      onTask={(t) => setModal({ type: "feedback", task: t })}
    />
  );
}
