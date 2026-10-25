import {
  CalendarDays,
  Database,
  Download,
  Leaf,
  Plus,
  ShieldCheck,
  Sparkles,
  Upload,
  X,
} from "lucide-react";
import { useState } from "react";
import * as api from "../../api";
import type { AiConfig, AppState, LifeRules } from "../../types";
import { errorText, uid } from "../../utils";

export default function SettingsPage({
  data,
  busy,
  saveRules,
  saveAi,
  action,
  toast,
}: {
  data: AppState;
  busy: boolean;
  saveRules: (r: LifeRules) => Promise<unknown>;
  saveAi: (c: AiConfig) => Promise<unknown>;
  action: (name: string, args?: Record<string, unknown>) => Promise<void>;
  toast: (s: string) => void;
}) {
  const [r, setR] = useState(data.rules),
    [c, setC] = useState(data.ai),
    [key, setKey] = useState("");
  return (
    <div className="settings-layout">
      <div>
        <section className="card settings-card">
          <div className="mini-title">
            <Leaf size={20} />
            <h2>生活边界与缓冲</h2>
          </div>
          <form
            className="form"
            onSubmit={(e) => {
              e.preventDefault();
              void saveRules(r);
            }}
          >
            <div className="form-grid">
              <label>
                一天从几点开始
                <input
                  type="number"
                  min={0}
                  max={23}
                  value={r.dayStart}
                  onChange={(e) => setR({ ...r, dayStart: +e.target.value })}
                />
              </label>
              <label>
                最晚安排到几点
                <input
                  type="number"
                  min={1}
                  max={24}
                  value={r.dayEnd}
                  onChange={(e) => setR({ ...r, dayEnd: +e.target.value })}
                />
              </label>
              <label>
                缓冲比例（%）
                <input
                  type="number"
                  min={0}
                  max={50}
                  value={r.bufferPercent}
                  onChange={(e) =>
                    setR({ ...r, bufferPercent: +e.target.value })
                  }
                />
              </label>
              <label>
                时区
                <input
                  value={r.timezone}
                  onChange={(e) => setR({ ...r, timezone: e.target.value })}
                />
              </label>
              <label>
                通常可用设备
                <input
                  value={r.equipment.join(",")}
                  onChange={(e) =>
                    setR({
                      ...r,
                      equipment: e.target.value
                        .split(/[,，]/)
                        .map((s) => s.trim())
                        .filter(Boolean),
                    })
                  }
                />
              </label>
              <label>
                默认地点
                <input
                  value={r.location}
                  onChange={(e) => setR({ ...r, location: e.target.value })}
                />
              </label>
            </div>
            <label>受保护的生活时间</label>
            {r.protected.map((p, i) => (
              <div className="protection-edit" key={p.id}>
                <input
                  aria-label="生活时间名称"
                  value={p.title}
                  onChange={(e) =>
                    setR({
                      ...r,
                      protected: r.protected.map((x, j) =>
                        i === j ? { ...x, title: e.target.value } : x,
                      ),
                    })
                  }
                />
                <input
                  aria-label="生活时间开始小时"
                  type="number"
                  min={0}
                  max={23}
                  value={p.startHour}
                  onChange={(e) =>
                    setR({
                      ...r,
                      protected: r.protected.map((x, j) =>
                        i === j ? { ...x, startHour: +e.target.value } : x,
                      ),
                    })
                  }
                />
                <span>至</span>
                <input
                  aria-label="生活时间结束小时"
                  type="number"
                  min={1}
                  max={24}
                  value={p.endHour}
                  onChange={(e) =>
                    setR({
                      ...r,
                      protected: r.protected.map((x, j) =>
                        i === j ? { ...x, endHour: +e.target.value } : x,
                      ),
                    })
                  }
                />
                <button
                  className="icon-button"
                  type="button"
                  aria-label="移除生活保障"
                  onClick={() =>
                    setR({
                      ...r,
                      protected: r.protected.filter((_, j) => j !== i),
                    })
                  }
                >
                  <X size={16} />
                </button>
              </div>
            ))}
            <button
              className="text-button"
              type="button"
              onClick={() =>
                setR({
                  ...r,
                  protected: [
                    ...r.protected,
                    {
                      id: uid(),
                      title: "休闲娱乐",
                      startHour: 18,
                      endHour: 19,
                      weekdays: [0, 1, 2, 3, 4, 5, 6],
                    },
                  ],
                })
              }
            >
              <Plus size={15} /> 添加每天的保留时间
            </button>
            <p className="form-note">
              这些时段不会被自动排程占用。修改后会检查锁定安排与其他冲突。
            </p>
            <button className="button primary" disabled={busy} type="submit">
              保存生活规则
            </button>
          </form>
        </section>
        <section className="card settings-card">
          <div className="mini-title">
            <Database size={20} />
            <h2>本地数据</h2>
          </div>
          <p>备份包含所有模块数据，不包含系统凭据中的 API 密钥。</p>
          <div className="data-actions">
            <button className="button" onClick={() => action("backup")}>
              <Download size={16} /> 导出备份
            </button>
            <button
              className="button"
              onClick={() => {
                if (
                  window.confirm(
                    "恢复备份会替换当前数据。系统会先保存恢复前备份。是否选择备份文件？",
                  )
                )
                  void action("restore");
              }}
            >
              <Upload size={16} /> 恢复备份
            </button>
            <button
              className="button"
              onClick={() => action("export_calendar")}
            >
              <CalendarDays size={16} /> 导出计划 ICS
            </button>
          </div>
        </section>
      </div>
      <section className="card settings-card ai-settings">
        <div className="mini-title">
          <Sparkles size={20} />
          <h2>云端 AI</h2>
        </div>
        <p>
          支持 Chat Completions 结构的 HTTPS
          接口。使用你自己的服务地址和模型，不预设供应商。
        </p>
        <form
          className="form"
          onSubmit={async (e) => {
            e.preventDefault();
            if (key) {
              try {
                await api.desktop("save_api_key", { key });
                setKey("");
              } catch (e) {
                toast(errorText(e));
                return;
              }
            }
            await saveAi(c);
          }}
        >
          <label className="toggle-row">
            <span>
              <b>启用 AI 辅助</b>
              <small>关闭后仍可本地排程</small>
            </span>
            <input
              type="checkbox"
              checked={c.enabled}
              onChange={(e) => setC({ ...c, enabled: e.target.checked })}
            />
          </label>
          <label>
            完整接口地址
            <input
              type="url"
              placeholder="https://你的服务/v1/chat/completions"
              value={c.endpoint}
              onChange={(e) => setC({ ...c, endpoint: e.target.value })}
            />
          </label>
          <label>
            模型名称
            <input
              placeholder="填写服务支持的模型标识"
              value={c.model}
              onChange={(e) => setC({ ...c, model: e.target.value })}
            />
          </label>
          <label>
            API 密钥
            <input
              type="password"
              autoComplete="off"
              placeholder="留空保留已有密钥"
              value={key}
              onChange={(e) => setKey(e.target.value)}
            />
          </label>
          <label>
            每日调用上限
            <input
              type="number"
              min={1}
              max={1000}
              value={c.maxCallsPerDay}
              onChange={(e) => setC({ ...c, maxCallsPerDay: +e.target.value })}
            />
          </label>
          <div className="form-note">
            <ShieldCheck size={16} /> 密钥存于 macOS
            钥匙串。任务上下文由你在每次请求中选择。
          </div>
          <button disabled={busy} type="submit" className="button primary">
            保存 AI 配置
          </button>
        </form>
      </section>
    </div>
  );
}
