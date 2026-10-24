import { useEffect, useState } from "react";

import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { useGuardedClose } from "@/hooks/useGuardedClose";
import { recordReservationDeposit, type ApiError, type Reservation } from "@/lib/api";
import { formatMoney, parseMoney } from "@/lib/format";
import { useSessionStore } from "@/stores/session";

interface Props {
  open: boolean;
  reservation: Reservation;
  onClose: () => void;
  onDone: (message: string) => void;
  onConflict: () => void;
}

/** Registro manual do sinal recebido (será substituído pelo módulo de recebimentos). */
export default function DepositDialog({ open, reservation, onClose, onDone, onConflict }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    setValue("");
    setError(null);
    setMessage(null);
  }, [open]);

  const { requestClose, confirm } = useGuardedClose(value !== "", onClose);

  const submit = async () => {
    setMessage(null);
    const cents = parseMoney(value);
    if (cents === null || cents <= 0) {
      setError("Informe um valor maior que zero, por exemplo 90,00.");
      return;
    }
    setError(null);
    setBusy(true);
    try {
      await recordReservationDeposit(token, reservation.id, cents, reservation.version);
      onDone(`Sinal de ${formatMoney(cents)} registrado.`);
    } catch (e) {
      const apiError = e as ApiError;
      setMessage(apiError.message);
      if (apiError.code === "conflict") onConflict();
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <Dialog open={open} onOpenChange={(next) => !next && requestClose()}>
        <DialogContent showCloseButton={false}>
          <DialogHeader>
            <DialogTitle>Registrar sinal recebido</DialogTitle>
            <DialogDescription>
              {`Esperado: ${formatMoney(reservation.deposit_expected_cents)}. Já registrado: ${formatMoney(reservation.deposit_paid_cents)}. Ao atingir o valor esperado, a reserva passa a confirmada.`}
            </DialogDescription>
          </DialogHeader>
          <FormField id="dep-value" label="Valor recebido (R$)" required error={error ?? undefined}>
            <Input id="dep-value" inputMode="decimal" value={value} onChange={(e) => setValue(e.target.value)} />
          </FormField>
          {message && (
            <p role="alert" className="text-sm text-destructive">
              {message}
            </p>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={requestClose}>
              Cancelar
            </Button>
            <Button disabled={busy} onClick={submit}>
              Registrar
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      {confirm}
    </>
  );
}
