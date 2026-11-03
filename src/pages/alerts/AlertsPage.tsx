import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { listAlerts, refreshAlerts, type AlertKind, type AlertSeverity, type AlertView } from "@/lib/api";
import { kindOptions, severityLabel } from "@/lib/alerts";
import { cn } from "@/lib/utils";
import { useSessionStore } from "@/stores/session";
import AlertList from "./AlertList";
import DismissDialog from "./DismissDialog";

const PAGE_SIZE = 20;
const selectClass = "h-8 rounded-lg border border-input bg-transparent px-2 text-sm";

/** Painel central de alertas: ativos por prioridade e histórico (nada é apagado). */
export default function AlertsPage() {
  const token = useSessionStore((s) => s.token) ?? "";
  const queryClient = useQueryClient();
  const [tab, setTab] = useState<"active" | "history">("active");
  const [kind, setKind] = useState<AlertKind | "">("");
  const [severity, setSeverity] = useState<AlertSeverity | "">("");
  const [page, setPage] = useState(1);
  const [dismissing, setDismissing] = useState<AlertView | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const query = { status: tab, ...(kind ? { kind } : {}), ...(severity ? { severity } : {}), page, page_size: PAGE_SIZE };
  const { data, isPending, isError } = useQuery({
    queryKey: ["alerts", query],
    refetchInterval: tab === "active" ? 60_000 : false,
    queryFn: async () => {
      if (tab === "active") {
        try {
          await refreshAlerts(token);
        } catch {
          // mostra o que já está salvo
        }
      }
      return listAlerts(token, query);
    },
  });

  const refreshAll = () => {
    void queryClient.invalidateQueries({ queryKey: ["alerts"] });
    void queryClient.invalidateQueries({ queryKey: ["alert-summary"] });
  };
  const done = (message: string) => {
    setDismissing(null);
    setNotice(message);
    refreshAll();
  };
  const reset = () => setPage(1);

  return (
    <section className="grid gap-4">
      <PageHeader title="Alertas" description="Pendências que exigem atenção. Ficam aqui até o problema ser resolvido." />
      {notice && (
        <p role="status" className="text-sm">
          {notice}
        </p>
      )}

      <div role="tablist" aria-label="Situação dos alertas" className="flex gap-2">
        {(
          [
            ["active", "Ativos"],
            ["history", "Histórico"],
          ] as const
        ).map(([value, label]) => (
          <Button
            key={value}
            role="tab"
            aria-selected={tab === value}
            variant={tab === value ? "default" : "outline"}
            size="sm"
            className={cn(tab === value && "font-semibold")}
            onClick={() => {
              setTab(value);
              reset();
            }}
          >
            {label}
          </Button>
        ))}
      </div>

      <div className="flex flex-wrap items-end gap-3">
        <label className="grid gap-1 text-sm">
          Tipo
          <select
            aria-label="Tipo"
            className={selectClass}
            value={kind}
            onChange={(e) => {
              setKind(e.target.value as AlertKind | "");
              reset();
            }}
          >
            <option value="">Todos</option>
            {kindOptions.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
        </label>
        <label className="grid gap-1 text-sm">
          Prioridade
          <select
            aria-label="Prioridade"
            className={selectClass}
            value={severity}
            onChange={(e) => {
              setSeverity(e.target.value as AlertSeverity | "");
              reset();
            }}
          >
            <option value="">Todas</option>
            {(Object.keys(severityLabel) as AlertSeverity[]).map((s) => (
              <option key={s} value={s}>
                {severityLabel[s]}
              </option>
            ))}
          </select>
        </label>
      </div>

      {isError && (
        <p role="alert" className="text-sm text-destructive">
          Não foi possível carregar os alertas. Tente de novo em instantes.
        </p>
      )}
      {!isPending && !isError && data?.items.length === 0 && (
        <p className="text-sm text-muted-foreground">
          {tab === "active"
            ? "Nenhum alerta ativo. Está tudo em ordem por enquanto."
            : "Nenhum alerta no histórico com esses filtros."}
        </p>
      )}
      {data && data.items.length > 0 && <AlertList alerts={data.items} onDismiss={setDismissing} />}
      {data && data.total > PAGE_SIZE && (
        <div className="flex items-center gap-2 text-sm">
          <Button variant="outline" size="sm" disabled={page <= 1} onClick={() => setPage(page - 1)}>
            Anterior
          </Button>
          <span>{`Página ${page} de ${Math.ceil(data.total / PAGE_SIZE)}`}</span>
          <Button variant="outline" size="sm" disabled={page * PAGE_SIZE >= data.total} onClick={() => setPage(page + 1)}>
            Próxima
          </Button>
        </div>
      )}
      {dismissing && <DismissDialog open alert={dismissing} onClose={() => setDismissing(null)} onDone={done} />}
    </section>
  );
}
