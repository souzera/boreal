import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Bar, BarChart, CartesianGrid, Cell, Line, LineChart, Pie, PieChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";

import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { managementDashboard, type ApiError } from "@/lib/api";
import { formatDate, formatMoney, toLocalIsoDate } from "@/lib/format";
import { useSessionStore } from "@/stores/session";
import { ChartCard } from "./ChartCard";

const COLORS = ["var(--chart-1)", "var(--chart-2)", "var(--chart-3)", "var(--chart-4)", "var(--chart-5)"];

type Preset = "hoje" | "7dias" | "mes" | "livre";

function addDays(iso: string, days: number): string {
  const [y, m, d] = iso.split("-").map(Number);
  return toLocalIsoDate(new Date(y, m - 1, d + days, 12));
}

function range(preset: Preset, today: string): { from: string; to: string } {
  if (preset === "7dias") return { from: addDays(today, -6), to: today };
  if (preset === "mes") return { from: `${today.slice(0, 8)}01`, to: today };
  return { from: today, to: today };
}

const percent = (bp: number) => `${(bp / 100).toFixed(1).replace(".", ",")}%`;
const shortDate = (iso: string) => `${iso.slice(8, 10)}/${iso.slice(5, 7)}`;

/** Visão gerencial (só administrador): números e gráficos do período, sempre com a tabela equivalente. */
export default function ManagementPage() {
  const token = useSessionStore((s) => s.token) ?? "";
  const isAdmin = useSessionStore((s) => s.user?.role === "admin");
  const today = toLocalIsoDate(new Date());
  const [preset, setPreset] = useState<Preset>("hoje");
  const [from, setFrom] = useState(today);
  const [to, setTo] = useState(today);

  const invalid = !from || !to ? "Informe as duas datas do período." : to < from ? "A data final não pode ser antes da inicial." : null;
  const { data, error } = useQuery({
    queryKey: ["management", from, to],
    queryFn: () => managementDashboard(token, from, to),
    enabled: !!token && isAdmin && !invalid,
    retry: false,
  });

  if (!isAdmin) {
    return (
      <section>
        <PageHeader title="Visão gerencial" description="Esta área é só para administradores." />
      </section>
    );
  }

  const choose = (next: Preset) => {
    const r = range(next, today);
    setPreset(next);
    setFrom(r.from);
    setTo(r.to);
  };
  const received = data?.receipts_by_method.reduce((sum, m) => sum + m.net_cents, 0) ?? 0;

  return (
    <section className="grid gap-4">
      <PageHeader title="Visão gerencial" description="Ocupação, recebimentos, saldos e contas do período." />

      <div className="flex flex-wrap items-end gap-2">
        {([["hoje", "Hoje"], ["7dias", "Últimos 7 dias"], ["mes", "Este mês"]] as const).map(([key, label]) => (
          <Button key={key} variant={preset === key ? "default" : "outline"} aria-pressed={preset === key} onClick={() => choose(key)}>
            {label}
          </Button>
        ))}
        <label className="grid gap-1 text-sm">
          De
          <Input type="date" value={from} onChange={(e) => { setPreset("livre"); setFrom(e.target.value); }} />
        </label>
        <label className="grid gap-1 text-sm">
          Até
          <Input type="date" value={to} onChange={(e) => { setPreset("livre"); setTo(e.target.value); }} />
        </label>
      </div>
      {invalid && (
        <p role="alert" className="text-sm text-destructive">
          {invalid}
        </p>
      )}
      {error && !invalid && (
        <p role="alert" className="text-sm text-destructive">
          {(error as unknown as ApiError).message}
        </p>
      )}

      {data && !invalid && (
        <>
          <ul aria-label="Números do período" className="grid grid-cols-2 gap-3 md:grid-cols-4">
            {[
              ["Chegadas", String(data.arrivals)],
              ["Saídas", String(data.departures)],
              ["Hospedados agora", String(data.in_house_now)],
              ["Recebido no período", formatMoney(received)],
              ["Saldos a receber", `${formatMoney(data.balances.to_receive_cents)} (${data.balances.to_receive_count})`],
              ["Créditos de hóspedes", `${formatMoney(data.balances.credit_cents)} (${data.balances.credit_count})`],
              ["Contas vencidas", `${formatMoney(data.payables.overdue_cents)} (${data.payables.overdue_count})`],
              ["Contas em aberto", formatMoney(data.payables.open_cents)],
            ].map(([label, value]) => (
              <li key={label} className="grid gap-0.5 rounded-xl border bg-card p-3 shadow-card">
                <span className="text-sm text-muted-foreground">{label}</span>
                <strong className="text-lg">{value}</strong>
              </li>
            ))}
          </ul>

          <div className="grid gap-4 lg:grid-cols-2">
            <ChartCard
              title="Ocupação por dia"
              description={`Percentual de quartos ocupados em cada dia, de ${formatDate(data.from)} a ${formatDate(data.to)}.`}
              empty={data.occupancy.every((o) => o.occupied === 0)}
              table={
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Data</TableHead>
                      <TableHead>Quartos ocupados</TableHead>
                      <TableHead>Ocupação</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {data.occupancy.map((o) => (
                      <TableRow key={o.date}>
                        <TableCell>{formatDate(o.date)}</TableCell>
                        <TableCell>{`${o.occupied} de ${o.rooms}`}</TableCell>
                        <TableCell>{percent(o.rate_bp)}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              }
            >
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={data.occupancy.map((o) => ({ dia: shortDate(o.date), taxa: o.rate_bp / 100 }))}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="dia" />
                  <YAxis unit="%" domain={[0, 100]} />
                  <Tooltip formatter={(v) => `${v}%`} />
                  <Line type="monotone" dataKey="taxa" name="Ocupação" stroke={COLORS[0]} strokeWidth={2} />
                </LineChart>
              </ResponsiveContainer>
            </ChartCard>

            <ChartCard
              title="Recebimentos por dia"
              description="Valor recebido menos o devolvido em cada dia."
              empty={data.receipts_daily.every((d) => d.net_cents === 0)}
              table={
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Data</TableHead>
                      <TableHead>Líquido</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {data.receipts_daily.map((d) => (
                      <TableRow key={d.date}>
                        <TableCell>{formatDate(d.date)}</TableCell>
                        <TableCell>{formatMoney(d.net_cents)}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              }
            >
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={data.receipts_daily.map((d) => ({ dia: shortDate(d.date), valor: d.net_cents / 100 }))}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="dia" />
                  <YAxis />
                  <Tooltip formatter={(v) => formatMoney(Math.round(Number(v) * 100))} />
                  <Bar dataKey="valor" name="Líquido" fill={COLORS[1]} />
                </BarChart>
              </ResponsiveContainer>
            </ChartCard>

            <ChartCard
              title="Contas em aberto por categoria"
              description="Quanto ainda falta pagar em cada categoria de despesa."
              empty={data.payables.by_category.length === 0}
              table={
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Categoria</TableHead>
                      <TableHead>Em aberto</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {data.payables.by_category.map((c) => (
                      <TableRow key={c.category_name}>
                        <TableCell>{c.category_name}</TableCell>
                        <TableCell>{formatMoney(c.open_cents)}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              }
            >
              <ResponsiveContainer width="100%" height="100%">
                <PieChart>
                  <Pie data={data.payables.by_category.map((c) => ({ nome: c.category_name, valor: c.open_cents / 100 }))} dataKey="valor" nameKey="nome" label={(p) => String(p.name)}>
                    {data.payables.by_category.map((c, i) => (
                      <Cell key={c.category_name} fill={COLORS[i % COLORS.length]} />
                    ))}
                  </Pie>
                  <Tooltip formatter={(v) => formatMoney(Math.round(Number(v) * 100))} />
                </PieChart>
              </ResponsiveContainer>
            </ChartCard>

            <section aria-label="Recebimentos por forma de pagamento" className="rounded-xl border bg-card p-4 shadow-card">
              <h2 className="mb-3 text-lg font-semibold">Recebimentos por forma de pagamento</h2>
              <div className="overflow-x-auto">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Forma</TableHead>
                      <TableHead>Recebido</TableHead>
                      <TableHead>Devolvido</TableHead>
                      <TableHead>Líquido</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {data.receipts_by_method.map((m) => (
                      <TableRow key={m.method}>
                        <TableCell>{m.label}</TableCell>
                        <TableCell>{formatMoney(m.received_cents)}</TableCell>
                        <TableCell>{formatMoney(m.refunded_cents)}</TableCell>
                        <TableCell>{formatMoney(m.net_cents)}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            </section>
          </div>
        </>
      )}
    </section>
  );
}
