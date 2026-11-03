import { useQuery } from "@tanstack/react-query";
import { save } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { useSearchParams } from "react-router-dom";

import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import {
  exportReportCsv,
  exportReportPdf,
  openExportedFile,
  runReport,
  type ApiError,
  type ReportKind,
  type ReportQuery,
} from "@/lib/api";
import { toLocalIsoDate } from "@/lib/format";
import { periodError, REPORTS, reportInfo, suggestedFileName } from "@/lib/report";
import { useSessionStore } from "@/stores/session";
import { ReportFilters, type Draft } from "./ReportFilters";
import { ReportTableView } from "./ReportTableView";

const DEFAULT_DAYS = 30;

function addDays(iso: string, days: number): string {
  const [y, m, d] = iso.split("-").map(Number);
  return toLocalIsoDate(new Date(y, m - 1, d + days, 12));
}

/** Pedido guardado na URL: recarregar a página repete a mesma consulta. */
function queryFromParams(params: URLSearchParams, kind: ReportKind, period: boolean): ReportQuery {
  const query: ReportQuery = { kind };
  const text = (key: string) => params.get(key) || undefined;
  if (period) {
    query.from = text("de");
    query.to = text("ate");
  }
  query.status = text("situacao");
  query.method = text("forma");
  query.text = text("texto");
  const room = Number(params.get("quarto"));
  if (room) query.room_id = room;
  const category = Number(params.get("categoria"));
  if (category) query.category_id = category;
  return query;
}

type Outcome = { kind: "saved"; path: string } | { kind: "error"; message: string; retry: () => void };

export default function ReportsPage() {
  const token = useSessionStore((s) => s.token) ?? "";
  const isAdmin = useSessionStore((s) => s.user?.role === "admin");
  const [params, setParams] = useSearchParams();
  const today = toLocalIsoDate(new Date());
  const allowed = REPORTS.filter((r) => isAdmin || !r.financial);

  const fromUrl = reportInfo(params.get("tipo"));
  const appliedInfo = fromUrl && allowed.includes(fromUrl) ? fromUrl : undefined;
  const applied = appliedInfo ? queryFromParams(params, appliedInfo.kind, appliedInfo.period) : null;

  const [kind, setKind] = useState<ReportKind>(appliedInfo?.kind ?? allowed[0].kind);
  const [draft, setDraft] = useState<Draft>({
    from: params.get("de") ?? addDays(today, -(DEFAULT_DAYS - 1)),
    to: params.get("ate") ?? today,
    status: params.get("situacao") ?? "",
    room: params.get("quarto") ?? "",
    method: params.get("forma") ?? "",
    category: params.get("categoria") ?? "",
    text: params.get("texto") ?? "",
  });
  const [formError, setFormError] = useState<string | null>(null);
  const [delimiter, setDelimiter] = useState<";" | ",">(";");
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  const [busy, setBusy] = useState(false);

  const info = reportInfo(kind) ?? allowed[0];
  const { data: table, error: queryError, isFetching } = useQuery({
    queryKey: ["report", applied],
    queryFn: () => runReport(token, applied as ReportQuery),
    enabled: !!token && !!applied,
    retry: false,
  });

  const choose = (next: ReportKind) => {
    setKind(next);
    setDraft((d) => ({ ...d, status: "", room: "", method: "", category: "", text: "" }));
    setFormError(null);
  };

  const submit = (event: React.FormEvent) => {
    event.preventDefault();
    if (info.period) {
      const problem = periodError(draft.from, draft.to);
      if (problem) {
        setFormError(problem);
        return;
      }
    }
    setFormError(null);
    setOutcome(null);
    const next = new URLSearchParams({ tipo: info.kind });
    const put = (key: string, value: string) => value && next.set(key, value);
    if (info.period) {
      put("de", draft.from);
      put("ate", draft.to);
    }
    if (info.filters.includes("status")) put("situacao", draft.status);
    if (info.filters.includes("room")) put("quarto", draft.room);
    if (info.filters.includes("method")) put("forma", draft.method);
    if (info.filters.includes("category")) put("categoria", draft.category);
    if (info.filters.includes("text")) put("texto", draft.text.trim());
    setParams(next);
  };

  const attempt = async (write: (path: string) => Promise<{ path: string }>, path: string) => {
    setBusy(true);
    try {
      const result = await write(path);
      setOutcome({ kind: "saved", path: result.path });
    } catch (e) {
      setOutcome({ kind: "error", message: (e as ApiError).message, retry: () => void attempt(write, path) });
    } finally {
      setBusy(false);
    }
  };

  /** Exporta a consulta **aplicada** (a que está na tela), não o que está sendo digitado. */
  const exportAs = async (extension: "csv" | "pdf") => {
    if (!applied) return;
    const path = await save({
      defaultPath: suggestedFileName(applied.kind, extension, today),
      filters: [{ name: extension.toUpperCase(), extensions: [extension] }],
    });
    if (!path) return;
    const write = (target: string) => (extension === "csv" ? exportReportCsv(token, applied, delimiter, target) : exportReportPdf(token, applied, target));
    await attempt(write, path);
  };

  const openSaved = async (path: string) => {
    try {
      await openExportedFile(token, path);
    } catch (e) {
      setOutcome({ kind: "error", message: (e as ApiError).message, retry: () => void openSaved(path) });
    }
  };

  return (
    <section className="grid gap-4">
      <PageHeader title="Relatórios" description="Consulte na tela, filtre e exporte em CSV ou PDF." />

      <form onSubmit={submit} className="grid gap-3 rounded-xl border bg-card p-4 shadow-card" aria-label="Filtros do relatório">
        <label className="grid max-w-md gap-1 text-sm">
          Relatório
          <select
            className="h-8 w-full rounded-lg border border-input bg-transparent px-2 text-sm"
            value={kind}
            onChange={(e) => choose(e.target.value as ReportKind)}
          >
            {allowed.map((r) => (
              <option key={r.kind} value={r.kind}>
                {r.label}
              </option>
            ))}
          </select>
          <span className="text-muted-foreground">{info.description}</span>
        </label>
        <ReportFilters report={info} draft={draft} onChange={(patch) => setDraft((d) => ({ ...d, ...patch }))} />
        {formError && (
          <p role="alert" className="text-sm text-destructive">
            {formError}
          </p>
        )}
        <div>
          <Button type="submit">Gerar relatório</Button>
        </div>
      </form>

      {queryError && (
        <p role="alert" className="text-sm text-destructive">
          {(queryError as unknown as ApiError).message}
        </p>
      )}

      {table && applied && (
        <section aria-label="Resultado" className="grid gap-3">
          <div>
            <h2 className="text-lg font-semibold">{table.title}</h2>
            <ul className="text-sm text-muted-foreground">
              {table.filters.map((f) => (
                <li key={f}>{f}</li>
              ))}
            </ul>
          </div>
          <div className="flex flex-wrap items-end gap-2">
            <label className="grid gap-1 text-sm">
              Separador do CSV
              <select
                className="h-8 rounded-lg border border-input bg-transparent px-2 text-sm"
                value={delimiter}
                onChange={(e) => setDelimiter(e.target.value as ";" | ",")}
              >
                <option value=";">Ponto e vírgula (Excel em português)</option>
                <option value=",">Vírgula</option>
              </select>
            </label>
            <Button variant="outline" disabled={busy || isFetching} onClick={() => void exportAs("csv")}>
              Exportar CSV
            </Button>
            <Button variant="outline" disabled={busy || isFetching} onClick={() => void exportAs("pdf")}>
              Exportar PDF
            </Button>
          </div>
          {outcome?.kind === "saved" && (
            <p role="status" className="flex flex-wrap items-center gap-2 text-sm">
              <span>{`Arquivo salvo em ${outcome.path}`}</span>
              <Button size="sm" variant="outline" onClick={() => void openSaved(outcome.path)}>
                Abrir arquivo
              </Button>
            </p>
          )}
          {outcome?.kind === "error" && (
            <p role="alert" className="flex flex-wrap items-center gap-2 text-sm text-destructive">
              <span>{outcome.message}</span>
              <Button size="sm" variant="outline" onClick={outcome.retry}>
                Tentar de novo
              </Button>
            </p>
          )}
          <ReportTableView table={table} />
        </section>
      )}
    </section>
  );
}
