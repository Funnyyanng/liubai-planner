import CanvasPage from "../modules/canvas/CanvasPage";
import {
  Blocks,
  CalendarDays,
  CheckCheck,
  Dumbbell,
  GitBranch,
  Settings,
  Sun,
} from "lucide-react";
import type { ComponentType } from "react";
import PlanningPage from "../modules/calendar/PlanningPage";
import ModulesPage from "../modules/modules/ModulesPage";
import AdjustmentsPage from "../modules/plans/AdjustmentsPage";
import RulesPage from "../modules/settings/RulesPage";
import ProjectsPage from "../modules/tasks/ProjectsPage";
import TrainingPage from "../modules/training/TrainingPage";
import type { Page, PageContext } from "./contracts";
export const pages: {
  id: Page;
  title: string;
  icon: typeof Sun;
  component: ComponentType<PageContext>;
}[] = [
  { id: "now", title: "现在", icon: Sun, component: PlanningPage },
  {
    id: "calendar",
    title: "日历",
    icon: CalendarDays,
    component: PlanningPage,
  },
  {
    id: "projects",
    title: "任务与项目",
    icon: CheckCheck,
    component: ProjectsPage,
  },
  {
    id: "adjustments",
    title: "调整中心",
    icon: GitBranch,
    component: AdjustmentsPage,
  },
  {
    id: "training",
    title: "训练计划",
    icon: Dumbbell,
    component: TrainingPage,
  },
  { id: "canvas", title: "Canvas", icon: CalendarDays, component: CanvasPage },
  { id: "modules", title: "模块中心", icon: Blocks, component: ModulesPage },
  { id: "settings", title: "规则与设置", icon: Settings, component: RulesPage },
];
export const attentionNames = {
  focused: "专注听课",
  partial: "部分可用",
  available: "自主可用",
};
