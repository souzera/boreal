import { useEffect, useState } from "react";

import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { useGuardedClose } from "@/hooks/useGuardedClose";
import { payPayable, type ApiError, type PayableView, type SelectableMethod } from "@/lib/api";
import { formatMoney, parseMoney } from "@/lib/format";
import { methodLabel, selectableMethods } from "@/lib/payment";
import { useSessionStore } from "@/stores/session";

interface Props {
  open: boolean;
  payable: PayableView;
  onClose: () => void;
  onDone: (message: string) => void;
  onConflict: () => void;
}

/** Pagamento parcial ou total de uma conta a pagar (spec §10.2). */
export default function PayPayableDialog({ open, payable, onClose, onDone, onConflict }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const [value, setValue] = useState("");
  const [method, setMethod] = useState<SelectableMethod | "">("");
  const [note, setNote] = useState("");
  const [errors, setErrors] = useState<{ value?: string; method?: string }>({});
  const [confirming, setConfirming] = useState<{ cents: number; method: SelectableMethod } | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    setValue("");
    setMethod("");
    setNote("");
    setErrors({});
    setConfirming(null);
    setMessage(null);
  }, [open]);

  const { requestClose, confirm } = useGuardedClose(value !== "" || note !== "", onClose);

  const review = () => {
    setMessage(null);
    const cents = parseMoney(value);
    const next: typeof errors = {};
    if (cents === null || cents <= 0) next.value = "Informe um valor maior que zero, por exemplo R$ 100,00.";
    else if (cents > payable.balance_cents) next.value = `O valor não pode passar do saldo da conta (${formatMoney(payable.balance_cents)}).`;
    if (method === "") next.method = "Escolha a forma de pagamento.";
    setErrors(next);
    if (Object.keys(next).length === 0 && cents !== null && method !== "") setConfirming({ cents, method });
  };

  const submit = async () => {
    if (!confirming) return;
    setBusy(true);
    try {
      await payPayable(token, payable.id, { amount_cents: confirming.cents, method: confirming.method, note: note.trim() }, payable.version);
      onDone(`Pagamento de ${formatMoney(confirming.cents)} da conta ${payable.code} registrado e salvo neste computador.`);
    } catch (e) {
      const apiError = e as ApiError;
      setConfirming(null);
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
            <DialogTitle>Registrar pagamento da conta</DialogTitle>
            <DialogDescription>{`${payable.supplier} · ${payable.code}. Saldo da conta: ${formatMoney(payable.balance_cents)}.`}</DialogDescription>
          </DialogHeader>
          {confirming === null ? (
            <div className="grid gap-3">
              <FormField id="pp-value" label="Valor pago" required error={errors.value}>
                <Input id="pp-value" inputMode="decimal" placeholder="R$ 0,00" value={value} onChange={(e) => setValue(e.target.value)} />
              </FormField>
              <FormField id="pp-method" label="Forma de pagamento" required error={errors.method}>
                <select
                  id="pp-method"
                  className="h-8 rounded-lg border border-input bg-transparent px-2 text-sm"
                  value={method}
                  onChange={(e) => setMethod(e.target.value as SelectableMethod | "")}
                >
                  <option value="">Escolha…</option>
                  {selectableMethods.map((m) => (
                    <option key={m} value={m}>
                      {methodLabel[m]}
                    </option>
                  ))}
                </select>
              </FormField>
              <FormField id="pp-note" label="Observação">
                <Input id="pp-note" value={note} onChange={(e) => setNote(e.target.value)} />
              </FormField>
            </div>
          ) : (
            <p className="text-sm font-medium">
              {`Registrar pagamento de ${formatMoney(confirming.cents)} em ${methodLabel[confirming.method]} para ${payable.supplier}? Depois de salvo, o pagamento não pode ser corrigido nesta tela.`}
            </p>
          )}
          {message && (
            <p role="alert" className="text-sm text-destructive">
              {message}
            </p>
          )}
          <DialogFooter>
            {confirming === null ? (
              <>
                <Button type="button" variant="outline" onClick={requestClose}>
                  Cancelar
                </Button>
                <Button type="button" onClick={review}>
                  Registrar pagamento da conta
                </Button>
              </>
            ) : (
              <>
                <Button type="button" variant="outline" onClick={() => setConfirming(null)}>
                  Corrigir dados
                </Button>
                <Button type="button" disabled={busy} onClick={() => void submit()}>
                  Confirmar pagamento da conta
                </Button>
              </>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>
      {confirm}
    </>
  );
}
