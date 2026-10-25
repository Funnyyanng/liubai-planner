import type { PageContext } from "../../shell/contracts";
import SettingsPage from "../settings/SettingsPage";
export default function RulesPage(props: PageContext) {
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
    <SettingsPage
      data={data}
      busy={busy}
      saveRules={(v) => run("saveRules", v, "生活规则已保存，计划已重新检查")}
      saveAi={(v) => run("configureAi", v, "AI 配置已保存")}
      action={action}
      toast={setToast}
    />
  );
}
