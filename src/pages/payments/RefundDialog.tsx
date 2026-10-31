import { useEffect, useState } from "react";

import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { useGuardedClose } from "@/hooks/useGuardedClose";
import { recordRefund, type ApiError } from "@/lib/api";
import { formatMoney, parseMoney } from "@/lib/format";
import { checkSplit, describeComponents, emptyRow, type SplitRow } from "@/lib/payment";
import { useSessionStore } from "@/stores/session";
import ComponentsEditor from "./ComponentsEditor";

interface Props {
  open: boolean;
  payment: { id: number; code: string; amount_cents: number; refundable_cents: number };
  onClose: () => void;
  onDone: (message: string) => void;
}

/** Devolução de parte ou do todo de um recebimento, com motivo (spec §9.2). */
export default function RefundDialog({ open, payment, onClose, onDone }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const [total, setTotal] = useState("");
  const [rows, setRows] = useState<SplitRow[]>([emptyRow()]);
  const [reason, setReason] = useState("");
  const [confirming, setConfirming] = useState(false);
  const [errors, setErrors] = useState<{ total?: string; split?: string; reason?: string }>({});
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    setTotal("");
    setRows([emptyRow()]);
    setReason("");
    setConfirming(false);
    setErrors({});
    setMessage(null);
  }, [open]);

  const { requestClose, confirm } = useGuardedClose(total !== "" || reason !== "", onClose);

  const totalCents = parseMoney(total);
  const check = checkSplit(totalCents, rows);
  const splitMismatch = rows.length > 1 && totalCents !== null && check.remaining !== 0;

  const review = () => {
    setMessage(null);
    const next: typeof errors = {};
    if (totalCents === null || totalCents <= 0) next.total = "Informe um valor maior que zero, por exemplo R$ 30,00.";
    else if (totalCents > payment.refundable_cents) next.total = `Só é possível devolver até ${formatMoney(payment.refundable_cents)} deste recebimento.`;
    else if (check.error) next.split = check.error;
    if (reason.trim() === "") next.reason = "Informe o motivo da devolução.";
    setErrors(next);
    if (Object.keys(next).length === 0) setConfirming(true);
  };

  const submit = async () => {
    if (!check.components) return;
    setBusy(true);
    try {
      const receipt = await recordRefund(token, { payment_id: payment.id, components: check.components, reason: reason.trim() });
      onDone(`Devolução ${receipt.code} de ${formatMoney(receipt.total_cents)} registrada e salva neste computador.`);
    } catch (e) {
      setConfirming(false);
      setMessage((e as ApiError).message);
    } finally {
      setBusy(false);
    }
  };

  const summary =
    check.components && check.components.length === 1
      ? `${formatMoney(check.sum)} em ${describeComponents(check.components).replace(/ R\$ .*$/, "")}`
      : `${formatMoney(check.sum)} dividido em ${check.components ? describeComponents(check.components) : ""}`;

  return (
    <>
      <Dialog open={open} onOpenChange={(next) => !next && requestClose()}>
        <DialogContent showCloseButton={false}>
          <DialogHeader>
            <DialogTitle>Registrar devolução</DialogTitle>
            <DialogDescription>
              {`Recebimento ${payment.code} de ${formatMoney(payment.amount_cents)}. Você pode devolver até ${formatMoney(payment.refundable_cents)}.`}
            </DialogDescription>
          </DialogHeader>
          {!confirming ? (
            <div className="grid gap-3">
              <FormField id="refund-total" label="Valor a devolver" required error={errors.total}>
                <Input id="refund-total" inputMode="decimal" placeholder="R$ 0,00" value={total} onChange={(e) => setTotal(e.target.value)} />
              </FormField>
              <ComponentsEditor totalCents={totalCents} rows={rows} onChange={setRows} error={errors.split} />
              <FormField id="refund-reason" label="Motivo da devolução" required error={errors.reason}>
                <Input id="refund-reason" value={reason} onChange={(e) => setReason(e.target.value)} />
              </FormField>
            </div>
          ) : (
            <p className="text-sm font-medium">
              {`Registrar devolução de ${summary}? Depois de salvo, a devolução não pode ser corrigida nesta tela.`}
            </p>
          )}
          {message && (
            <p role="alert" className="text-sm text-destructive">
              {message}
            </p>
          )}
          <DialogFooter>
            {!confirming ? (
              <>
                <Button type="button" variant="outline" onClick={requestClose}>
                  Cancelar
                </Button>
                <Button type="button" disabled={splitMismatch} onClick={review}>
                  Registrar devolução
                </Button>
              </>
            ) : (
              <>
                <Button type="button" variant="outline" onClick={() => setConfirming(false)}>
                  Corrigir dados
                </Button>
                <Button type="button" disabled={busy} onClick={() => void submit()}>
                  Confirmar devolução
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
