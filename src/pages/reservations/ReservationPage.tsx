import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { Link, useLocation, useParams } from "react-router-dom";

import { ConfirmDialog } from "@/components/ConfirmDialog";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Textarea } from "@/components/ui/textarea";
import { usePermissions } from "@/hooks/usePermissions";
import {
  confirmReservation,
  getReservation,
  markReservationNoShow,
  setReservationNotes,
  type ApiError,
} from "@/lib/api";
import { formatDate, formatMoney, formatTime, toLocalIsoDate } from "@/lib/format";
import { isOpen, ruleLabel, statusIcon, statusLabel } from "@/lib/reservation";
import { useSessionStore } from "@/stores/session";
import CancelDialog from "./CancelDialog";
import ChangeDialog from "./ChangeDialog";
import DepositDialog from "./DepositDialog";
import DiscountDialog from "./DiscountDialog";

type Dialogs = "change" | "cancel" | "discount" | "deposit" | "no_show" | null;

const kindLabel: Record<string, string> = {
  change: "Alteração de datas ou quarto",
  discount: "Desconto",
  cancel: "Cancelamento",
  no_show: "No-show",
  confirm: "Confirmação",
  deposit: "Sinal recebido",
  notes: "Observações",
};

function Line({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-wrap justify-between gap-2">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="font-medium">{children}</dd>
    </div>
  );
}

export default function ReservationPage() {
  const { id } = useParams();
  const reservationId = Number(id);
  const token = useSessionStore((s) => s.token) ?? "";
  const location = useLocation();
  const queryClient = useQueryClient();
  const { can } = usePermissions();
  const [dialog, setDialog] = useState<Dialogs>(null);
  const [notice, setNotice] = useState<string | null>((location.state as { notice?: string } | null)?.notice ?? null);
  const [error, setError] = useState<string | null>(null);
  const [editingNotes, setEditingNotes] = useState(false);
  const [notes, setNotes] = useState("");

  const queryKey = ["reservation", reservationId];
  const { data, isError } = useQuery({ queryKey, queryFn: () => getReservation(token, reservationId) });

  const refresh = () => {
    void queryClient.invalidateQueries({ queryKey });
    void queryClient.invalidateQueries({ queryKey: ["reservations"] });
    void queryClient.invalidateQueries({ queryKey: ["reservations-review"] });
  };
  const done = (message: string) => {
    setDialog(null);
    setError(null);
    setNotice(message);
    refresh();
  };

  if (isError) {
    return (
      <section>
        <PageHeader title="Reserva" description="Não foi possível carregar a reserva." />
        <Button render={<Link to="/reservas" />} variant="outline">
          Voltar para as reservas
        </Button>
      </section>
    );
  }
  if (!data) return null;

  const { reservation: r, nights, history } = data;
  const open = isOpen(r.status);
  const today = toLocalIsoDate(new Date());
  // A tolerância de chegada é aplicada pelo sistema ao marcar; a tela oferece a ação a partir do dia da chegada.
  const arrivalPassed = r.check_in_date <= today;

  const run = async (action: () => Promise<unknown>, success: string) => {
    setError(null);
    try {
      await action();
      done(success);
    } catch (e) {
      const apiError = e as ApiError;
      setError(apiError.message);
      setDialog(null);
      if (apiError.code === "conflict") refresh();
    }
  };

  return (
    <section className="grid gap-6">
      <PageHeader
        title={`Reserva ${r.code}`}
        description={`${statusIcon[r.status]} ${statusLabel[r.status]}`}
        action={
          <Button render={<Link to="/reservas" />} variant="outline">
            Voltar
          </Button>
        }
      />
      {notice && (
        <p role="status" className="rounded-lg border border-primary/30 bg-primary/5 p-3 text-sm">
          {notice}
        </p>
      )}
      {error && (
        <p role="alert" className="rounded-lg border border-destructive/40 bg-destructive/5 p-3 text-sm text-destructive">
          {error}
        </p>
      )}
      {r.deposit_overdue && (
        <p className="w-fit rounded border border-destructive/40 px-2 py-1 text-sm font-medium text-destructive">Sinal vencido</p>
      )}

      <div className="grid gap-6 md:grid-cols-2">
        <dl className="grid gap-2 text-sm">
          <Line label="Hóspede">
            <Link to={`/hospedes/${r.guest_id}`} className="underline">
              {r.guest_name}
            </Link>
          </Line>
          <Line label="Quarto">{`Quarto ${r.room_code}`}</Line>
          <Line label="Período">{`${formatDate(r.check_in_date)} a ${formatDate(r.check_out_date)} (${r.nights} ${r.nights === 1 ? "noite" : "noites"})`}</Line>
          <Line label="Horários previstos">{`Entrada ${r.expected_check_in_time} · saída ${r.expected_check_out_time}`}</Line>
          <Line label="Pessoas">{String(r.occupants)}</Line>
        </dl>
        <dl className="grid gap-2 text-sm">
          <Line label="Valor de tabela">{formatMoney(r.list_total_cents)}</Line>
          <Line label={r.discount_reason ? `Desconto (${r.discount_reason})` : "Desconto"}>{`− ${formatMoney(r.discount_cents)}`}</Line>
          <Line label="Valor combinado">{formatMoney(r.agreed_total_cents)}</Line>
          <p>{`Sinal esperado: ${formatMoney(r.deposit_expected_cents)}`}</p>
          <p>{`Sinal recebido: ${formatMoney(r.deposit_paid_cents)}`}</p>
          <p>{`Prazo do sinal: ${formatDate(r.deposit_due_date)}`}</p>
          {r.cancellation_fee_cents !== null && <p>{`Retido no cancelamento: ${formatMoney(r.cancellation_fee_cents)}`}</p>}
        </dl>
      </div>

      <div className="flex flex-wrap gap-2">
        {open && can("change_reservation") && <Button onClick={() => setDialog("change")}>Alterar datas ou quarto</Button>}
        {open && can("apply_discount") && (
          <Button variant="outline" onClick={() => setDialog("discount")}>
            Aplicar desconto
          </Button>
        )}
        {r.status === "pending" && (
          <Button variant="outline" onClick={() => run(() => confirmReservation(token, r.id, r.version), "Reserva confirmada.")}>
            Confirmar reserva
          </Button>
        )}
        {open && (
          <Button variant="outline" onClick={() => setDialog("deposit")}>
            Registrar sinal recebido
          </Button>
        )}
        {open && arrivalPassed && (
          <Button variant="outline" onClick={() => setDialog("no_show")}>
            Marcar no-show
          </Button>
        )}
        {open && (
          <Button variant="destructive" onClick={() => setDialog("cancel")}>
            Cancelar reserva
          </Button>
        )}
      </div>

      <section aria-label="Noites">
        <h2 className="mb-2 text-base font-semibold">Noites</h2>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Noite</TableHead>
              <TableHead>Diária</TableHead>
              <TableHead>Regra aplicada</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {nights.map((n) => (
              <TableRow key={n.date}>
                <TableCell>{formatDate(n.date)}</TableCell>
                <TableCell>{formatMoney(n.amount_cents)}</TableCell>
                <TableCell>{ruleLabel[n.rule]}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </section>

      <section aria-label="Observações">
        <h2 className="mb-2 text-base font-semibold">Observações</h2>
        {editingNotes ? (
          <div className="grid gap-2">
            <Textarea aria-label="Texto das observações" value={notes} onChange={(e) => setNotes(e.target.value)} />
            <div className="flex gap-2">
              <Button
                onClick={() =>
                  run(async () => {
                    await setReservationNotes(token, r.id, notes, r.version);
                    setEditingNotes(false);
                  }, "Observações salvas.")
                }
              >
                Salvar observações
              </Button>
              <Button variant="outline" onClick={() => setEditingNotes(false)}>
                Cancelar edição
              </Button>
            </div>
          </div>
        ) : (
          <div className="flex flex-wrap items-start gap-3">
            <p className="text-sm">{r.notes || "Sem observações."}</p>
            {open && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => {
                  setNotes(r.notes);
                  setEditingNotes(true);
                }}
              >
                Editar observações
              </Button>
            )}
          </div>
        )}
      </section>

      <section aria-label="Histórico">
        <h2 className="mb-2 text-base font-semibold">Histórico</h2>
        {history.length === 0 ? (
          <p className="text-sm text-muted-foreground">Nenhuma alteração registrada.</p>
        ) : (
          <ul className="grid gap-2 text-sm">
            {history.map((h) => (
              <li key={h.id} className="rounded-lg border p-3">
                <p className="font-medium">{kindLabel[h.kind] ?? h.kind}</p>
                <p className="text-muted-foreground">{`${formatDate(h.changed_at.slice(0, 10))} ${formatTime(h.changed_at)} · ${h.changed_by_name}`}</p>
                {h.price_before_cents !== null && h.price_after_cents !== null && (
                  <p>{`Valor: ${formatMoney(h.price_before_cents)} → ${formatMoney(h.price_after_cents)}`}</p>
                )}
                {h.justification && <p>{h.justification}</p>}
              </li>
            ))}
          </ul>
        )}
      </section>

      <ChangeDialog
        open={dialog === "change"}
        reservation={r}
        onClose={() => setDialog(null)}
        onApplied={done}
        onConflict={refresh}
      />
      <CancelDialog
        open={dialog === "cancel"}
        reservation={r}
        canOverride={can("cancellation_exception")}
        onClose={() => setDialog(null)}
        onDone={done}
        onConflict={refresh}
      />
      <DiscountDialog open={dialog === "discount"} reservation={r} onClose={() => setDialog(null)} onDone={done} onConflict={refresh} />
      <DepositDialog open={dialog === "deposit"} reservation={r} onClose={() => setDialog(null)} onDone={done} onConflict={refresh} />
      <ConfirmDialog
        open={dialog === "no_show"}
        title="Marcar como no-show?"
        description="O hóspede não chegou. O sinal pago fica retido e o quarto é liberado."
        confirmLabel="Confirmar no-show"
        cancelLabel="Voltar"
        destructive
        onConfirm={() => run(() => markReservationNoShow(token, r.id, r.version), "Reserva marcada como no-show.")}
        onCancel={() => setDialog(null)}
      />
    </section>
  );
}
