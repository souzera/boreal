import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";

import Icon from "@/components/Icon";
import { useAlertSummary } from "@/hooks/useAlertSummary";
import { cleaningQueue } from "@/lib/api";
import { severityClass, severityIcon, severityLabel } from "@/lib/alerts";
import { cn } from "@/lib/utils";
import { formatDate } from "@/lib/format";
import { useSessionStore } from "@/stores/session";

function todayIso(): string {
  const d = new Date();
  const month = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${month}-${day}`;
}

export default function Dashboard() {
  const token = useSessionStore((s) => s.token) ?? "";
  const { data: queue } = useQuery({ queryKey: ["cleaning-queue"], queryFn: () => cleaningQueue(token), enabled: !!token });

  const { data: alerts } = useAlertSummary();

  return (
    <section>
      <h1 className="text-[26px] font-[650] leading-tight">Recepção</h1>
      <p className="text-sm text-muted-foreground">Hoje, {formatDate(todayIso())}</p>
      <section aria-label="Alertas" className="mt-4 grid max-w-2xl gap-2">
        <h2 className="text-lg font-semibold">Alertas</h2>
        {alerts && alerts.top.length === 0 && <p className="text-sm text-muted-foreground">Nenhum alerta ativo. Está tudo em ordem.</p>}
        {alerts && alerts.top.length > 0 && (
          <ul className="grid gap-2">
            {alerts.top.map((a) => (
              <li key={a.id}>
                <Link to={a.link} className="grid gap-0.5 rounded-xl border bg-card p-3 text-sm shadow-card hover:bg-muted">
                  <span className="flex flex-wrap items-center gap-2">
                    <span className={cn("inline-flex items-center gap-1 rounded-md border px-2 py-0.5 text-xs font-medium", severityClass[a.severity])}>
                      <Icon name={severityIcon[a.severity]} size="xs" />
                      {severityLabel[a.severity]}
                    </span>
                    <strong>{a.title}</strong>
                  </span>
                  <span className="text-muted-foreground">{a.detail}</span>
                </Link>
              </li>
            ))}
          </ul>
        )}
        <Link to="/alertas" className="w-fit text-sm underline underline-offset-2">
          Ver todos os alertas
        </Link>
      </section>
      {queue &&
        (queue.length > 0 ? (
          <Link to="/limpeza" className="mt-4 inline-block rounded-lg border px-3 py-2 text-sm font-medium underline-offset-2 hover:underline">
            {`Quartos aguardando limpeza: ${queue.length}`}
          </Link>
        ) : (
          <p className="mt-4 text-sm text-muted-foreground">Nenhum quarto aguardando limpeza.</p>
        ))}
    </section>
  );
}
