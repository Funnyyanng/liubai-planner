import { Leaf } from "lucide-react";
import { type ReactNode } from "react";
export default function Empty({
  title,
  body,
  action,
}: {
  title: string;
  body: string;
  action?: ReactNode;
}) {
  return (
    <div className="empty-state">
      <Leaf size={30} />
      <h3>{title}</h3>
      <p>{body}</p>
      {action}
    </div>
  );
}
