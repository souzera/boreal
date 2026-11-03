import { useEffect, useState } from "react";

import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { useGuardedClose } from "@/hooks/useGuardedClose";
import { reverseStayCharge, type ApiError, type ChargeLine } from "@/lib/api";
import { formatMoney, parseMoney } from "@/lib/format";
import { useSessionStore } from "@/stores/session";

interface Props {
  open: boolean;
  stayId: number;
  charge: ChargeLine | null;
  onClose: () => void;
  onDone: (message: string) => void;
}

/** Estorna um lançamento (o original fica no histórico) e, se quiser, lança de novo com o valor certo. */
export default function ReverseDialog({ open, stayId, charge, onClose, onDone }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const [justification, setJustification] = useState("");
  const [relaunch, setRelaunch] = useState(false);
  const [description, setDescription] = useState("");
  const [value, setValue] = useState("");
  const [errors, setErrors] = useState<Partial<Record<"justification" | "description" | "value", string>>>({});
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open || !charge) return;
    setJustification("");
    setRelaunch(false);
    setDescription(charge.description);
    setValue(formatMoney(charge.amount_cents));
    setErrors({});
    setMessage(null);
  }, [open, charge]);

  const { requestClose, confirm } = useGuardedClose(justification !== "", onClose);
  if (!charge) return null;
  const canRelaunch = charge.kind === "consumption" || charge.kind === "extra";

  const submit = async () => {
    setMessage(null);
    const next: typeof errors = {};
    if (!justification.trim()) next.justification = "Informe a justificativa do estorno.";
    const cents = parseMoney(value);
    if (relaunch) {
      if (!description.trim()) next.description = "Informe a nova descrição.";
      if (cents === null || cents <= 0) next.value = "Informe um valor maior que zero, por exemplo R$ 35,00.";
    }
    setErrors(next);
    if (Object.keys(next).length > 0) return;

    setBusy(true);
    try {
      const replacement =
        relaunch && canRelaunch && cents !== null
          ? { kind: charge.kind as "consumption" | "extra", description: description.trim(), amount_cents: cents, item_id: null }
          : null;
      await reverseStayCharge(token, stayId, charge.id, justification.trim(), replacement);
      onDone(replacement ? "Lançamento estornado e lançado novamente." : "Lançamento estornado.");
    } catch (e) {
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
            <DialogTitle>Estornar lançamento</DialogTitle>
            <DialogDescription>O lançamento original continua no histórico, marcado como estornado, e o saldo é corrigido.</DialogDescription>
          </DialogHeader>
          <div className="grid gap-3">
            <p className="rounded-lg border p-2 text-sm font-medium">{`${charge.description} · ${formatMoney(charge.amount_cents)}`}</p>
            <FormField id="rv-justification" label="Justificativa do estorno" required error={errors.justification}>
              <Textarea id="rv-justification" value={justification} onChange={(e) => setJustification(e.target.value)} />
            </FormField>
            {canRelaunch && (
              <label className="flex items-center gap-2 text-sm">
                <input type="checkbox" checked={relaunch} onChange={(e) => setRelaunch(e.target.checked)} />
                Relançar com novo valor ou descrição
              </label>
            )}
            {relaunch && (
              <>
                <FormField id="rv-description" label="Nova descrição" required error={errors.description}>
                  <Input id="rv-description" value={description} onChange={(e) => setDescription(e.target.value)} />
                </FormField>
                <FormField id="rv-value" label="Novo valor" required error={errors.value}>
                  <Input id="rv-value" inputMode="decimal" value={value} onChange={(e) => setValue(e.target.value)} />
                </FormField>
              </>
            )}
            {message && (
              <p role="alert" className="text-sm text-destructive">
                {message}
              </p>
            )}
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={requestClose}>
              Cancelar
            </Button>
            <Button type="button" variant="destructive" disabled={busy} onClick={submit}>
              Estornar lançamento
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      {confirm}
    </>
  );
}
