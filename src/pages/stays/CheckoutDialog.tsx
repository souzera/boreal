import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Textarea } from "@/components/ui/textarea";
import { usePermissions } from "@/hooks/usePermissions";
import { checkoutStay, previewStayCheckout, type ApiError, type ChargeKind, type CheckoutPreview, type LateFeeChoice } from "@/lib/api";
import { formatMoney } from "@/lib/format";
import { useSessionStore } from "@/stores/session";

interface Props {
  open: boolean;
  stayId: number;
  onClose: () => void;
  onDone: (message: string) => void;
}

type Choice = "charge" | "waive" | null;

/** Soma dos lançamentos vigentes (sem os estornados nem os próprios estornos) das categorias pedidas. */
function total(preview: CheckoutPreview, kinds: ChargeKind[]): number {
  return preview.account.charges.filter((c) => kinds.includes(c.kind) && !c.reversed).reduce((sum, c) => sum + c.amount_cents, 0);
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between gap-4 text-sm">
      <span>{label}</span>
      <span className="font-medium">{value}</span>
    </div>
  );
}

/** Demonstrativo e conclusão do check-out: quitação, saída tardia e exceção de saldo. */
export default function CheckoutDialog({ open, stayId, onClose, onDone }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const { can } = usePermissions();
  const [choice, setChoice] = useState<Choice>(null);
  const [waiveReason, setWaiveReason] = useState("");
  const [useException, setUseException] = useState(false);
  const [balanceReason, setBalanceReason] = useState("");
  const [errors, setErrors] = useState<Partial<Record<"waive" | "balance", string>>>({});
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const { data: preview, refetch } = useQuery({
    queryKey: ["checkout-preview", stayId],
    queryFn: () => previewStayCheckout(token, stayId),
    enabled: open,
    gcTime: 0,
    staleTime: 0,
  });

  useEffect(() => {
    if (!open) return;
    setChoice(null);
    setWaiveReason("");
    setUseException(false);
    setBalanceReason("");
    setErrors({});
    setMessage(null);
  }, [open]);

  if (!preview) {
    return (
      <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
        <DialogContent showCloseButton={false}>
          <DialogHeader>
            <DialogTitle>Concluir check-out</DialogTitle>
            <DialogDescription>Carregando o demonstrativo…</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose}>
              Voltar
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  }

  const { late, risk, account } = preview;
  const balance = late.applies && choice === "charge" ? preview.balance_with_late_fee_cents : preview.balance_cents;
  const lateDecided = !late.applies || choice !== null;
  const mayException = can("checkout_with_balance");
  const needsException = balance !== 0;
  const blocked = !lateDecided || (needsException && (!mayException || !useException));

  const submit = async () => {
    setMessage(null);
    const next: typeof errors = {};
    if (choice === "waive" && !waiveReason.trim()) next.waive = "Informe a justificativa para dispensar a cobrança.";
    if (needsException && useException && !balanceReason.trim()) next.balance = "Informe a justificativa do saldo em aberto.";
    setErrors(next);
    if (Object.keys(next).length > 0) return;

    const lateFee: LateFeeChoice = !late.applies ? "not_applicable" : choice === "charge" ? "charge" : { waive: { justification: waiveReason.trim() } };
    setBusy(true);
    try {
      await checkoutStay(token, {
        stay_id: stayId,
        expected_version: preview.stay.version,
        late_fee: lateFee,
        balance_exception: needsException ? { justification: balanceReason.trim() } : null,
      });
      onDone("Check-out concluído e salvo neste computador.");
    } catch (e) {
      const apiError = e as ApiError;
      setMessage(apiError.message);
      // A conta mudou desde a prévia: mostra os números atuais para a recepção revisar.
      if (apiError.code === "conflict") void refetch();
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>{`Concluir check-out · quarto ${preview.stay.room_code}`}</DialogTitle>
          <DialogDescription>{`${preview.stay.guest_name}. Confira o demonstrativo antes de concluir.`}</DialogDescription>
        </DialogHeader>
        <div className="grid gap-4">
          <section aria-label="Demonstrativo" className="grid gap-1 rounded-lg border p-3">
            <h3 className="text-sm font-semibold">Demonstrativo</h3>
            <Row label="Diárias" value={formatMoney(total(preview, ["night"]))} />
            <Row label="Consumos e despesas extras" value={formatMoney(total(preview, ["consumption", "extra"]))} />
            <Row label="Descontos e ajustes" value={`${total(preview, ["discount", "adjustment"]) < 0 ? "− " : ""}${formatMoney(Math.abs(total(preview, ["discount", "adjustment"])))}`} />
            {total(preview, ["late_fee"]) > 0 && <Row label="Saída tardia" value={formatMoney(total(preview, ["late_fee"]))} />}
            <Row label="Recebimentos" value={formatMoney(account.received_cents)} />
            <Row label="Devoluções" value={formatMoney(account.refunded_cents)} />
          </section>

          {late.applies && (
            <fieldset className="grid gap-2 rounded-lg border border-destructive/40 p-3 text-sm">
              <legend className="px-1 font-medium">Saída tardia</legend>
              <p>{`⚠ Saída ${late.minutes_late} min depois do previsto; cobrança prevista ${formatMoney(late.fee_cents)}.`}</p>
              <p className="text-muted-foreground">Nenhuma diária é criada automaticamente.</p>
              <label className="flex items-center gap-2">
                <input type="radio" name="late-choice" checked={choice === "charge"} onChange={() => setChoice("charge")} />
                Cobrar saída tardia
              </label>
              {can("waive_late_fee") && (
                <label className="flex items-center gap-2">
                  <input type="radio" name="late-choice" checked={choice === "waive"} onChange={() => setChoice("waive")} />
                  Dispensar cobrança
                </label>
              )}
              {choice === "waive" && (
                <FormField id="co-waive" label="Justificativa para dispensar" required error={errors.waive}>
                  <Textarea id="co-waive" value={waiveReason} onChange={(e) => setWaiveReason(e.target.value)} />
                </FormField>
              )}
            </fieldset>
          )}

          <section aria-label="Saldo" className="grid gap-2 text-sm">
            {balance === 0 ? (
              <p className="font-medium">Conta quitada.</p>
            ) : (
              <>
                <p className="font-medium">{balance > 0 ? `Saldo em aberto: ${formatMoney(balance)}` : `Valor a devolver ao hóspede ${formatMoney(-balance)}`}</p>
                {mayException ? (
                  <>
                    <label className="flex items-center gap-2">
                      <input type="checkbox" checked={useException} onChange={(e) => setUseException(e.target.checked)} />
                      Concluir com saldo em aberto
                    </label>
                    {useException && (
                      <FormField id="co-balance" label="Justificativa do saldo em aberto" required error={errors.balance}>
                        <Textarea id="co-balance" value={balanceReason} onChange={(e) => setBalanceReason(e.target.value)} />
                      </FormField>
                    )}
                  </>
                ) : (
                  <p className="text-muted-foreground">
                    Registre o pagamento antes de concluir. Peça a quem pode autorizar o check-out com saldo pendente.
                  </p>
                )}
              </>
            )}
          </section>

          {risk && (
            <p role="alert" className="rounded-lg border border-destructive/40 bg-destructive/5 p-2 text-sm">
              {`⚠ Esta saída pode atrasar a chegada da reserva ${risk.next_code} (≈ ${risk.overrun_minutes} min).`}
            </p>
          )}
          {message && (
            <p role="alert" className="text-sm text-destructive">
              {message}
            </p>
          )}
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={onClose}>
            Voltar
          </Button>
          <Button type="button" disabled={busy || blocked} onClick={submit}>
            Concluir check-out
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
