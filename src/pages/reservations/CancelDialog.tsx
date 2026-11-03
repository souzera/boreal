import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import {
  cancelReservation,
  previewReservationCancellation,
  type ApiError,
  type CancellationOverride,
  type Reservation,
} from "@/lib/api";
import { formatMoney, parseMoney } from "@/lib/format";
import { useSessionStore } from "@/stores/session";

interface Props {
  open: boolean;
  reservation: Reservation;
  /** Pode aplicar exceção à política (`CancellationException`). */
  canOverride: boolean;
  onClose: () => void;
  onDone: (message: string) => void;
  onConflict: () => void;
}

function notice(days: number): string {
  if (days > 1) return `Faltam ${days} dias para a chegada.`;
  if (days === 1) return "Falta 1 dia para a chegada.";
  if (days === 0) return "A chegada é hoje.";
  return `A chegada era há ${-days} dia(s).`;
}

/** Cancelamento com o cálculo da política à vista; exceções só com permissão e justificativa. */
export default function CancelDialog({ open, reservation, canOverride, onClose, onDone, onConflict }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const { data: preview } = useQuery({
    queryKey: ["cancellation-preview", reservation.id, reservation.version],
    queryFn: () => previewReservationCancellation(token, reservation.id),
    enabled: open,
  });
  const [override, setOverride] = useState(false);
  const [retained, setRetained] = useState("");
  const [why, setWhy] = useState("");
  const [errors, setErrors] = useState<{ retained?: string; why?: string }>({});
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    setOverride(false);
    setRetained("");
    setWhy("");
    setErrors({});
    setMessage(null);
  }, [open]);

  const submit = async () => {
    if (!preview) return;
    setMessage(null);
    let exception: CancellationOverride | null = null;
    if (override) {
      const next: { retained?: string; why?: string } = {};
      const cents = parseMoney(retained);
      if (cents === null || cents < 0 || cents > preview.deposit_paid_cents) {
        next.retained = `O valor retido deve estar entre R$ 0,00 e o sinal pago (${formatMoney(preview.deposit_paid_cents)}).`;
      }
      if (why.trim() === "") next.why = "Informe a justificativa da exceção.";
      setErrors(next);
      if (Object.keys(next).length > 0 || cents === null) return;
      exception = { retained_cents: cents, justification: why.trim() };
    }
    setBusy(true);
    try {
      const result = await cancelReservation(token, reservation.id, reservation.version, exception);
      onDone(`Reserva cancelada. Devolução: ${formatMoney(result.refund_cents)}; retenção: ${formatMoney(result.retained_cents)}.`);
    } catch (e) {
      const error = e as ApiError;
      setMessage(error.message);
      if (error.code === "conflict") onConflict();
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>Cancelar reserva {reservation.code}?</DialogTitle>
          <DialogDescription>O registro é preservado e o quarto fica livre para novas reservas.</DialogDescription>
        </DialogHeader>
        {preview && (
          <div className="grid gap-1 text-sm">
            <p>{notice(preview.days_before)}</p>
            <p>{`Sinal já pago: ${formatMoney(preview.deposit_paid_cents)}`}</p>
            <p className="font-medium">{`Devolução: ${formatMoney(preview.refund_cents)}`}</p>
            <p className="font-medium">{`Retenção: ${formatMoney(preview.retained_cents)}`}</p>
            {!preview.can_cancel && (
              <p role="alert" className="text-destructive">
                Esta reserva não pode mais ser cancelada.
              </p>
            )}
          </div>
        )}
        {canOverride && preview && (
          <div className="grid gap-3">
            <label className="flex items-center gap-2 text-sm">
              <input type="checkbox" checked={override} onChange={(e) => setOverride(e.target.checked)} />
              Aplicar exceção à política
            </label>
            {override && (
              <>
                <FormField id="cx-retained" label="Valor retido (R$)" required error={errors.retained}>
                  <Input id="cx-retained" inputMode="decimal" value={retained} onChange={(e) => setRetained(e.target.value)} />
                </FormField>
                <FormField id="cx-why" label="Justificativa" required error={errors.why}>
                  <Textarea id="cx-why" value={why} onChange={(e) => setWhy(e.target.value)} />
                </FormField>
              </>
            )}
          </div>
        )}
        {message && (
          <p role="alert" className="text-sm text-destructive">
            {message}
          </p>
        )}
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            Voltar
          </Button>
          <Button variant="destructive" disabled={!preview?.can_cancel || busy} onClick={submit}>
            Confirmar cancelamento
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
