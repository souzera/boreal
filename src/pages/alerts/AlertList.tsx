import { Link } from "react-router-dom";

import Icon from "@/components/Icon";
import { Button } from "@/components/ui/button";
import type { AlertView } from "@/lib/api";
import { formatInstant } from "@/lib/format";
import { severityClass, severityIcon, severityLabel } from "@/lib/alerts";
import { cn } from "@/lib/utils";

interface Props {
  alerts: AlertView[];
  onDismiss: (alert: AlertView) => void;
}

function resolution(alert: AlertView): string {
  if (alert.status === "dismissed") {
    return `Dispensado por ${alert.resolved_by_name ?? "alguém"} em ${alert.resolved_at ? formatInstant(alert.resolved_at) : "—"}: ${alert.resolution_note ?? ""}`;
  }
  if (alert.status === "resolved") {
    return `Resolvido automaticamente em ${alert.resolved_at ? formatInstant(alert.resolved_at) : "—"}: o problema deixou de existir.`;
  }
  return "";
}

/** Lista de alertas: o que houve, desde quando, o registro de origem e, se couber, a dispensa. */
export default function AlertList({ alerts, onDismiss }: Props) {
  return (
    <ul className="grid gap-3">
      {alerts.map((a) => (
        <li key={a.id} className="grid gap-2 rounded-xl border bg-card p-4 shadow-card">
          <div className="flex flex-wrap items-center gap-2">
            <span className={cn("inline-flex items-center gap-1 rounded-md border px-2 py-0.5 text-xs font-medium", severityClass[a.severity])}>
              <Icon name={severityIcon[a.severity]} size="xs" />
              {severityLabel[a.severity]}
            </span>
            <span className="text-xs text-muted-foreground">{a.kind_label}</span>
          </div>
          <h3 className="text-[15px] font-semibold">{a.title}</h3>
          <p className="text-sm">{a.detail}</p>
          <p className="text-xs text-muted-foreground">{`Visto desde ${formatInstant(a.first_seen_at)}`}</p>
          {a.status !== "active" && <p className="text-sm">{resolution(a)}</p>}
          {a.status === "active" && (
            <div className="flex flex-wrap items-center gap-2">
              <Button size="sm" render={<Link to={a.link} />}>
                Abrir registro
              </Button>
              {a.severity === "critical" ? (
                <span className="text-xs text-muted-foreground">Este alerta só se resolve quando o problema for resolvido.</span>
              ) : (
                <Button size="sm" variant="outline" onClick={() => onDismiss(a)}>
                  Dispensar alerta
                </Button>
              )}
            </div>
          )}
        </li>
      ))}
    </ul>
  );
}
