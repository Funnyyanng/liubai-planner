import { type ReactNode } from "react";
export default function Stat({
  label,
  value,
  detail,
  icon,
  warn = false,
}: {
  label: string;
  value: string;
  detail: string;
  icon: ReactNode;
  warn?: boolean;
}) {
  return (
    <div className={`stat ${warn ? "warn" : ""}`}>
      <div>
        <span>{label}</span>
        <span className="stat-icon">{icon}</span>
      </div>
      <strong>{value}</strong>
      <small>{detail}</small>
    </div>
  );
}
