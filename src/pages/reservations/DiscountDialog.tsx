import { useEffect, useState } from "react";

import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { useGuardedClose } from "@/hooks/useGuardedClose";
import { applyReservationDiscount, type ApiError, type Reservation } from "@/lib/api";
import { formatMoney, parseMoney } from "@/lib/format";
import { useSessionStore } from "@/stores/session";

interface Props {
  open: boolean;
  reservation: Reservation;
  onClose: () => void;
  onDone: (message: string) => void;
  onConflict: () => void;
}

export default function DiscountDialog({ open, reservation, onClose, onDone, onConflict }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const [kind, setKind] = useState<"amount" | "percent">("amount");
  const [value, setValue] = useState("");
  const [why, setWhy] = useState("");
  const [errors, setErrors] = useState<{ value?: string; why?: string }>({});
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    setKind("amount");
    setValue("");
    setWhy("");
    setErrors({});
    setMessage(null);
  }, [open]);

  const { requestClose, confirm } = useGuardedClose(value !== "" || why !== "", onClose);

  const submit = async () => {
    setMessage(null);
    const next: { value?: string; why?: string } = {};
    let amountCents: number | null = null;
    let percentBp: number | null = null;
    if (kind === "amount") {
      const cents = parseMoney(value);
      if (value.trim() === "") next.value = "Informe o valor do desconto.";
      else if (cents === null || cents <= 0) next.value = "Informe um valor válido, por exemplo 25,00.";
      else if (cents > reservation.list_total_cents) next.value = `O desconto não pode ser maior que ${formatMoney(reservation.list_total_cents)}.`;
      amountCents = cents;
    } else {
      const percent = Number(value.replace(",", "."));
      if (value.trim() === "") next.value = "Informe o percentual do desconto.";
      else if (!Number.isFinite(percent) || percent <= 0 || percent > 100) next.value = "Informe um percentual de 0,01% a 100%.";
      percentBp = Math.round(percent * 100);
    }
    if (why.trim() === "") next.why = "Informe a justificativa do desconto.";
    setErrors(next);
    if (Object.keys(next).length > 0) return;

    setBusy(true);
    try {
      await applyReservationDiscount(
        token,
        reservation.id,
        { kind, amount_cents: kind === "amount" ? amountCents : null, percent_bp: kind === "percent" ? percentBp : null, justification: why.trim() },
        reservation.version,
      );
      onDone("Desconto aplicado.");
    } catch (e) {
      const error = e as ApiError;
      setMessage(error.message);
      if (error.code === "conflict") onConflict();
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <Dialog open={open} onOpenChange={(next) => !next && requestClose()}>
        <DialogContent showCloseButton={false}>
          <DialogHeader>
            <DialogTitle>Aplicar desconto</DialogTitle>
            <DialogDescription>
              {`Calculado sobre o valor de tabela (${formatMoney(reservation.list_total_cents)}). Substitui o desconto atual, se houver.`}
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4">
            <fieldset className="flex gap-4 text-sm">
              <legend className="sr-only">Tipo de desconto</legend>
              <label className="flex items-center gap-2">
                <input type="radio" name="discount-kind" checked={kind === "amount"} onChange={() => { setKind("amount"); setValue(""); }} />
                Em reais
              </label>
              <label className="flex items-center gap-2">
                <input type="radio" name="discount-kind" checked={kind === "percent"} onChange={() => { setKind("percent"); setValue(""); }} />
                Em percentual
              </label>
            </fieldset>
            <FormField
              id="disc-value"
              label={kind === "amount" ? "Valor do desconto (R$)" : "Percentual do desconto (%)"}
              required
              error={errors.value}
            >
              <Input id="disc-value" inputMode="decimal" value={value} onChange={(e) => setValue(e.target.value)} />
            </FormField>
            <FormField id="disc-why" label="Justificativa" required error={errors.why}>
              <Textarea id="disc-why" value={why} onChange={(e) => setWhy(e.target.value)} />
            </FormField>
            {message && (
              <p role="alert" className="text-sm text-destructive">
                {message}
              </p>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={requestClose}>
              Cancelar
            </Button>
            <Button disabled={busy} onClick={submit}>
              Aplicar
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      {confirm}
    </>
  );
}
