import { useEffect, useState } from "react";

import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { useGuardedClose } from "@/hooks/useGuardedClose";
import { recordStayPayment, type ApiError, type PaymentInput } from "@/lib/api";
import { formatMoney, parseMoney } from "@/lib/format";
import { balanceLabel, paymentMethodLabel, selectableMethods } from "@/lib/stay";
import { useSessionStore } from "@/stores/session";

interface Props {
  open: boolean;
  stayId: number;
  balanceCents: number;
  onClose: () => void;
  onDone: (message: string) => void;
}

/** Recebimento provisório (uma forma de pagamento): será substituído pelo módulo financeiro. */
export default function PaymentDialog({ open, stayId, balanceCents, onClose, onDone }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const [value, setValue] = useState("");
  const [method, setMethod] = useState<PaymentInput["method"] | "">("");
  const [note, setNote] = useState("");
  const [errors, setErrors] = useState<Partial<Record<"value" | "method", string>>>({});
  const [message, setMessage] = useState<string | null>(null);
  const [confirming, setConfirming] = useState<{ cents: number; method: PaymentInput["method"] } | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    setValue("");
    setMethod("");
    setNote("");
    setErrors({});
    setMessage(null);
    setConfirming(null);
  }, [open]);

  const { requestClose, confirm } = useGuardedClose(value !== "" || note !== "", onClose);

  const review = () => {
    setMessage(null);
    const next: typeof errors = {};
    const cents = parseMoney(value);
    if (cents === null || cents <= 0) next.value = "Informe um valor maior que zero, por exemplo R$ 100,00.";
    if (method === "") next.method = "Escolha a forma de pagamento.";
    setErrors(next);
    if (Object.keys(next).length > 0 || cents === null || method === "") return;
    setConfirming({ cents, method });
  };

  const submit = async () => {
    if (!confirming) return;
    setBusy(true);
    try {
      await recordStayPayment(token, stayId, { amount_cents: confirming.cents, method: confirming.method, note: note.trim() });
      onDone(`Pagamento de ${formatMoney(confirming.cents)} registrado.`);
    } catch (e) {
      setConfirming(null);
      setMessage((e as ApiError).message);
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <Dialog open={open} onOpenChange={(next) => !next && requestClose()}>
        <DialogContent showCloseButton={false}>
          <DialogHeader>
            <DialogTitle>Registrar pagamento</DialogTitle>
            <DialogDescription>{`${balanceLabel(balanceCents)}. Registro provisório: será substituído pelo módulo financeiro.`}</DialogDescription>
          </DialogHeader>
          {confirming === null ? (
            <div className="grid gap-3">
              <FormField id="pay-value" label="Valor recebido" required error={errors.value}>
                <Input id="pay-value" inputMode="decimal" placeholder="R$ 0,00" value={value} onChange={(e) => setValue(e.target.value)} />
              </FormField>
              <FormField id="pay-method" label="Forma de pagamento" required error={errors.method}>
                <select
                  id="pay-method"
                  className="h-8 rounded-lg border border-input bg-transparent px-2 text-sm"
                  value={method}
                  onChange={(e) => setMethod(e.target.value as PaymentInput["method"] | "")}
                >
                  <option value="">Escolha…</option>
                  {selectableMethods.map((m) => (
                    <option key={m} value={m}>
                      {paymentMethodLabel[m]}
                    </option>
                  ))}
                </select>
              </FormField>
              <FormField id="pay-note" label="Observação">
                <Input id="pay-note" value={note} onChange={(e) => setNote(e.target.value)} />
              </FormField>
            </div>
          ) : (
            <p className="text-sm font-medium">
              {`Registrar pagamento de ${formatMoney(confirming.cents)} em ${paymentMethodLabel[confirming.method]}? Depois de salvo, o valor não pode ser corrigido nesta tela.`}
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
                  Registrar pagamento
                </Button>
              </>
            ) : (
              <>
                <Button type="button" variant="outline" onClick={() => setConfirming(null)}>
                  Corrigir dados
                </Button>
                <Button type="button" disabled={busy} onClick={submit}>
                  Confirmar pagamento
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
