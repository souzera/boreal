import { useQuery } from "@tanstack/react-query";

import { Input } from "@/components/ui/input";
import { listExpenseCategories, listRooms } from "@/lib/api";
import { METHOD_OPTIONS, STATUS_OPTIONS, type ReportInfo } from "@/lib/report";
import { useSessionStore } from "@/stores/session";

const selectClass = "h-8 w-full rounded-lg border border-input bg-transparent px-2 text-sm";

export interface Draft {
  from: string;
  to: string;
  status: string;
  room: string;
  method: string;
  category: string;
  text: string;
}

interface Props {
  report: ReportInfo;
  draft: Draft;
  onChange: (patch: Partial<Draft>) => void;
}

/** Só os filtros que o relatório escolhido aceita (o backend recusa os demais). */
export function ReportFilters({ report, draft, onChange }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const wantsRooms = report.filters.includes("room");
  const wantsCategories = report.filters.includes("category");
  const rooms = useQuery({
    queryKey: ["report-rooms"],
    queryFn: () => listRooms(token, { page: 1, page_size: 100, sort: "code", direction: "asc" }),
    enabled: !!token && wantsRooms,
  });
  const categories = useQuery({
    queryKey: ["report-categories"],
    queryFn: () => listExpenseCategories(token, false),
    enabled: !!token && wantsCategories,
  });
  const statuses = STATUS_OPTIONS[report.kind] ?? [];

  return (
    <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
      {report.period ? (
        <>
          <label className="grid gap-1 text-sm">
            De
            <Input type="date" value={draft.from} onChange={(e) => onChange({ from: e.target.value })} />
          </label>
          <label className="grid gap-1 text-sm">
            Até
            <Input type="date" value={draft.to} onChange={(e) => onChange({ to: e.target.value })} />
          </label>
        </>
      ) : (
        <p className="self-end text-sm text-muted-foreground">Mostra a situação de agora; não usa período.</p>
      )}
      {report.filters.includes("status") && (
        <label className="grid gap-1 text-sm">
          Situação
          <select className={selectClass} value={draft.status} onChange={(e) => onChange({ status: e.target.value })}>
            <option value="">Todas</option>
            {statuses.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
        </label>
      )}
      {wantsRooms && (
        <label className="grid gap-1 text-sm">
          Quarto
          <select className={selectClass} value={draft.room} onChange={(e) => onChange({ room: e.target.value })}>
            <option value="">Todos</option>
            {(rooms.data?.items ?? []).map((r) => (
              <option key={r.id} value={String(r.id)}>
                {r.code}
              </option>
            ))}
          </select>
        </label>
      )}
      {report.filters.includes("method") && (
        <label className="grid gap-1 text-sm">
          Forma de pagamento
          <select className={selectClass} value={draft.method} onChange={(e) => onChange({ method: e.target.value })}>
            <option value="">Todas</option>
            {METHOD_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
        </label>
      )}
      {wantsCategories && (
        <label className="grid gap-1 text-sm">
          Categoria
          <select className={selectClass} value={draft.category} onChange={(e) => onChange({ category: e.target.value })}>
            <option value="">Todas</option>
            {(categories.data ?? []).map((c) => (
              <option key={c.id} value={String(c.id)}>
                {c.name}
              </option>
            ))}
          </select>
        </label>
      )}
      {report.filters.includes("text") && (
        <label className="grid gap-1 text-sm">
          Texto
          <Input value={draft.text} placeholder="Hóspede, código ou fornecedor" onChange={(e) => onChange({ text: e.target.value })} />
        </label>
      )}
    </div>
  );
}
