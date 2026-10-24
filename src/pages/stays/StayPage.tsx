import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Link, useParams } from "react-router-dom";

import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { usePermissions } from "@/hooks/usePermissions";
import { getStay } from "@/lib/api";
import { formatDate, formatInstant, formatMoney } from "@/lib/format";
import { balanceLabel, chargeKindLabel, paymentMethodLabel, stayStatusIcon, stayStatusLabel } from "@/lib/stay";
import { useSessionStore } from "@/stores/session";

export type StayDialog = "charge" | "payment" | "checkout" | { reverse: number } | null;

function Line({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-wrap justify-between gap-2">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="font-medium">{children}</dd>
    </div>
  );
}

export default function StayPage() {
  const { id } = useParams();
  const stayId = Number(id);
  const token = useSessionStore((s) => s.token) ?? "";
  const { can } = usePermissions();
  const [, setDialog] = useState<StayDialog>(null);
  const { data, isError } = useQuery({ queryKey: ["stay", stayId], queryFn: () => getStay(token, stayId) });

  if (isError) {
    return (
      <section>
        <PageHeader title="Hospedagem" description="Não foi possível carregar a hospedagem." />
        <Button render={<Link to="/hospedagens" />} variant="outline">
          Voltar para as hospedagens
        </Button>
      </section>
    );
  }
  if (!data) return null;

  const { stay, occupants, account } = data;
  const inHouse = stay.status === "in_house";

  return (
    <section className="grid gap-6">
      <PageHeader
        title={`Hospedagem ${stay.code}`}
        description={`${stayStatusIcon[stay.status]} ${stayStatusLabel[stay.status]}`}
        action={
          <div className="flex gap-2">
            <Button render={<Link to={`/reservas/${stay.reservation_id}`} />} variant="outline">
              Ver reserva
            </Button>
            <Button render={<Link to="/hospedagens" />} variant="outline">
              Voltar
            </Button>
          </div>
        }
      />

      <div className="grid gap-6 md:grid-cols-2">
        <dl className="grid gap-2 text-sm">
          <Line label="Hóspede">
            <Link to={`/hospedes/${stay.guest_id}`} className="underline">
              {stay.guest_name}
            </Link>
          </Line>
          <Line label="Quarto">{`Quarto ${stay.room_code}`}</Line>
          <Line label="Período previsto">{`${formatDate(stay.check_in_date)} a ${formatDate(stay.check_out_date)}`}</Line>
          <Line label="Horários previstos">{`Entrada ${stay.expected_check_in_time} · saída ${stay.expected_check_out_time}`}</Line>
          <Line label="Check-in feito em">{formatInstant(stay.checked_in_at)}</Line>
          {stay.early_check_in && <Line label="Observação">Check-in antecipado autorizado</Line>}
          {stay.checked_out_at && <Line label="Check-out feito em">{formatInstant(stay.checked_out_at)}</Line>}
        </dl>
        <section aria-label="Ocupantes">
          <h2 className="mb-2 text-base font-semibold">Ocupantes</h2>
          <ul className="grid gap-1 text-sm">
            {occupants.map((o) => (
              <li key={o.id}>
                {o.full_name}
                {o.is_holder && <span className="text-muted-foreground"> (titular)</span>}
                {o.document && <span className="text-muted-foreground">{` · ${o.document}`}</span>}
              </li>
            ))}
          </ul>
        </section>
      </div>

      <p className="w-fit rounded-lg border px-3 py-2 text-lg font-semibold">{balanceLabel(account.balance_cents)}</p>

      {inHouse && (
        <div className="flex flex-wrap gap-2">
          <Button onClick={() => setDialog("charge")}>Lançar consumo</Button>
          <Button variant="outline" onClick={() => setDialog("payment")}>
            Registrar pagamento
          </Button>
          <Button variant="outline" onClick={() => setDialog("checkout")}>
            Concluir check-out
          </Button>
        </div>
      )}

      <section aria-label="Conta">
        <h2 className="mb-2 text-base font-semibold">Conta da hospedagem</h2>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Tipo</TableHead>
              <TableHead>Descrição</TableHead>
              <TableHead>Valor</TableHead>
              <TableHead>Situação</TableHead>
              <TableHead>Lançado por</TableHead>
              {inHouse && <TableHead>Ações</TableHead>}
            </TableRow>
          </TableHeader>
          <TableBody>
            {account.charges.map((c) => (
              <TableRow key={c.id}>
                <TableCell>{chargeKindLabel[c.kind]}</TableCell>
                <TableCell className={c.reversed ? "line-through" : undefined}>
                  {c.description}
                  {c.justification && <span className="block text-xs text-muted-foreground">{c.justification}</span>}
                </TableCell>
                <TableCell className={c.reversed ? "line-through" : undefined}>{formatMoney(c.amount_cents)}</TableCell>
                <TableCell>{c.reversed ? <span className="text-xs font-medium">Estornado</span> : "—"}</TableCell>
                <TableCell>{`${c.created_by_name} · ${formatInstant(c.created_at)}`}</TableCell>
                {inHouse && (
                  <TableCell>
                    {can("reverse_entry") && c.kind !== "reversal" && !c.reversed && (
                      <Button size="sm" variant="outline" onClick={() => setDialog({ reverse: c.id })}>
                        Estornar lançamento
                      </Button>
                    )}
                  </TableCell>
                )}
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </section>

      <section aria-label="Recebimentos">
        <h2 className="mb-2 text-base font-semibold">Recebimentos e devoluções</h2>
        {account.payments.length === 0 && account.refunds.length === 0 ? (
          <p className="text-sm text-muted-foreground">Nenhum recebimento registrado.</p>
        ) : (
          <ul className="grid gap-1 text-sm">
            {account.payments.map((p) => (
              <li key={p.id}>
                <span className="font-medium">{p.code}</span>
                {` · ${paymentMethodLabel[p.method]} · ${formatMoney(p.amount_cents)} · ${formatInstant(p.received_at)} · ${p.received_by_name}`}
              </li>
            ))}
            {account.refunds.map((f) => (
              <li key={f.id}>
                <span className="font-medium">{f.code}</span>
                {` · devolução de ${f.payment_code} · ${formatMoney(f.amount_cents)} · ${f.reason}`}
              </li>
            ))}
          </ul>
        )}
      </section>
    </section>
  );
}
