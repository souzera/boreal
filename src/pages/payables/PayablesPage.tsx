import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { DataTable, type ColumnDef } from "@/components/data-table/DataTable";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { usePermissions } from "@/hooks/usePermissions";
import {
  listExpenseCategories,
  listPayables,
  payablesSummary,
  type PayableStatus,
  type PayableView,
} from "@/lib/api";
import { formatDate, formatMoney, toLocalIsoDate } from "@/lib/format";
import { useSessionStore } from "@/stores/session";
import CategoriesPanel from "./CategoriesPanel";
import PayableDialog from "./PayableDialog";
import PayPayableDialog from "./PayPayableDialog";

const PAGE_SIZE = 20;

/** Ícone junto do rótulo: a situação nunca depende só de cor. */
const statusIcon: Record<PayableStatus, string> = { pending: "○", partial: "◐", paid: "●", overdue: "⚠" };

const statusOptions: { value: PayableStatus; label: string }[] = [
  { value: "pending", label: "Pendente" },
  { value: "partial", label: "Parcial" },
  { value: "paid", label: "Paga" },
  { value: "overdue", label: "Vencida" },
];

const selectClass = "h-8 rounded-lg border border-input bg-transparent px-2 text-sm";

function monthRange(): { from: string; to: string } {
  const now = new Date();
  const first = new Date(now.getFullYear(), now.getMonth(), 1);
  const last = new Date(now.getFullYear(), now.getMonth() + 1, 0);
  return { from: toLocalIsoDate(first), to: toLocalIsoDate(last) };
}

export default function PayablesPage() {
  const token = useSessionStore((s) => s.token) ?? "";
  const isAdmin = useSessionStore((s) => s.user?.role === "admin");
  const queryClient = useQueryClient();
  const { can, ready } = usePermissions();
  const allowed = ready && can("manage_payables");
  const [status, setStatus] = useState<PayableStatus | "">("");
  const [category, setCategory] = useState("");
  const [page, setPage] = useState(1);
  const [creating, setCreating] = useState(false);
  const [paying, setPaying] = useState<PayableView | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [basis, setBasis] = useState<"competence" | "payment">("competence");
  const [range, setRange] = useState(monthRange);

  const query = {
    ...(status ? { status } : {}),
    ...(category ? { category_id: Number(category) } : {}),
    page,
    page_size: PAGE_SIZE,
  };
  const list = useQuery({ queryKey: ["payables", query], queryFn: () => listPayables(token, query), enabled: allowed });
  const categories = useQuery({ queryKey: ["expense-categories"], queryFn: () => listExpenseCategories(token, false), enabled: allowed });
  const summary = useQuery({
    queryKey: ["payables-summary", basis, range],
    queryFn: () => payablesSummary(token, { ...range, basis }),
    enabled: allowed,
  });
  const refresh = () => {
    void queryClient.invalidateQueries({ queryKey: ["payables"] });
    void queryClient.invalidateQueries({ queryKey: ["payables-summary"] });
  };
  const done = (message: string) => {
    setCreating(false);
    setPaying(null);
    setNotice(message);
    refresh();
  };

  if (!ready) return null;
  if (!allowed) {
    return (
      <section>
        <PageHeader title="Contas a pagar" description="Despesas e obrigações da pousada." />
        <p role="alert" className="text-sm">
          Você não tem permissão para ver as contas a pagar. Peça a um administrador para liberar o acesso.
        </p>
      </section>
    );
  }

  const columns: ColumnDef<PayableView>[] = [
    { key: "code", header: "Código", render: (p) => p.code },
    { key: "supplier", header: "Fornecedor", render: (p) => p.supplier },
    { key: "description", header: "Descrição", render: (p) => p.description },
    { key: "category", header: "Categoria", render: (p) => p.category_name },
    { key: "due", header: "Vencimento", render: (p) => formatDate(p.due_date) },
    { key: "amount", header: "Valor", render: (p) => formatMoney(p.amount_cents) },
    { key: "paid", header: "Pago", render: (p) => formatMoney(p.paid_cents) },
    { key: "balance", header: "Saldo", render: (p) => formatMoney(p.balance_cents) },
    {
      key: "status",
      header: "Situação",
      render: (p) => (
        <span className={p.status === "overdue" ? "font-medium text-destructive" : undefined}>
          <span aria-hidden="true">{statusIcon[p.status]} </span>
          {p.status_label}
        </span>
      ),
    },
    {
      key: "actions",
      header: "Ações",
      render: (p) =>
        p.balance_cents > 0 ? (
          <Button size="sm" variant="outline" onClick={() => setPaying(p)}>
            Registrar pagamento da conta
          </Button>
        ) : null,
    },
  ];

  return (
    <section className="grid gap-6">
      <PageHeader
        title="Contas a pagar"
        description="Despesas e obrigações da pousada, com fornecedor, vencimento e categoria."
        action={<Button onClick={() => setCreating(true)}>Registrar conta a pagar</Button>}
      />
      {notice && (
        <p role="status" className="text-sm">
          {notice}
        </p>
      )}

      <div className="flex flex-wrap items-end gap-3">
        <label className="grid gap-1 text-sm">
          Situação
          <select
            aria-label="Situação"
            className={selectClass}
            value={status}
            onChange={(e) => {
              setStatus(e.target.value as PayableStatus | "");
              setPage(1);
            }}
          >
            <option value="">Todas</option>
            {statusOptions.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
        </label>
        <label className="grid gap-1 text-sm">
          Categoria
          <select
            aria-label="Categoria"
            className={selectClass}
            value={category}
            onChange={(e) => {
              setCategory(e.target.value);
              setPage(1);
            }}
          >
            <option value="">Todas</option>
            {(categories.data ?? []).map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </select>
        </label>
      </div>

      <DataTable
        columns={columns}
        rows={list.data?.items ?? []}
        getRowKey={(p) => p.id}
        total={list.data?.total ?? 0}
        page={page}
        pageSize={PAGE_SIZE}
        emptyMessage="Nenhuma conta a pagar encontrada."
        onPageChange={setPage}
        onSortChange={() => undefined}
      />

      <section aria-label="Totais por categoria">
        <h2 className="mb-2 text-base font-semibold">Totais por categoria</h2>
        <div className="mb-2 flex flex-wrap items-end gap-3">
          <div role="radiogroup" aria-label="Visão" className="flex gap-3 text-sm">
            {(
              [
                ["competence", "Competência"],
                ["payment", "Pagamento"],
              ] as const
            ).map(([value, label]) => (
              <label key={value} className="flex items-center gap-1">
                <input type="radio" name="basis" checked={basis === value} onChange={() => setBasis(value)} />
                {label}
              </label>
            ))}
          </div>
          <label className="grid gap-1 text-sm">
            De
            <Input type="date" value={range.from} onChange={(e) => setRange({ ...range, from: e.target.value })} />
          </label>
          <label className="grid gap-1 text-sm">
            Até
            <Input type="date" value={range.to} onChange={(e) => setRange({ ...range, to: e.target.value })} />
          </label>
        </div>
        <p className="mb-1 text-xs text-muted-foreground">
          {basis === "competence"
            ? "Pela data da despesa (competência)."
            : "Pela data em que cada pagamento foi feito; pagamentos parciais entram pelo valor e data de cada um."}
        </p>
        {(summary.data ?? []).length === 0 ? (
          <p className="text-sm text-muted-foreground">Nenhuma despesa no período.</p>
        ) : (
          <ul className="grid gap-1 text-sm">
            {(summary.data ?? []).map((c) => (
              <li key={c.category_id}>{`${c.category_name} ${formatMoney(c.total_cents)}`}</li>
            ))}
          </ul>
        )}
      </section>

      {isAdmin && <CategoriesPanel categories={categories.data ?? []} />}

      <PayableDialog open={creating} categories={categories.data ?? []} onClose={() => setCreating(false)} onDone={done} />
      {paying && <PayPayableDialog open payable={paying} onClose={() => setPaying(null)} onDone={done} onConflict={refresh} />}
    </section>
  );
}
