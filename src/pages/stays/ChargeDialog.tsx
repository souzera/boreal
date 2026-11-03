import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { useGuardedClose } from "@/hooks/useGuardedClose";
import { usePermissions } from "@/hooks/usePermissions";
import { addStayAdjustment, addStayCharge, listItems, type ApiError } from "@/lib/api";
import { formatMoney, parseMoney } from "@/lib/format";
import { useSessionStore } from "@/stores/session";

type Kind = "consumption" | "extra" | "adjustment";

interface Props {
  open: boolean;
  stayId: number;
  onClose: () => void;
  onDone: (message: string) => void;
}

const selectClass = "h-8 rounded-lg border border-input bg-transparent px-2 text-sm";

/** Lança consumo, despesa extra (avulsos ou do catálogo) ou ajuste da conta (com permissão). */
export default function ChargeDialog({ open, stayId, onClose, onDone }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const { can } = usePermissions();
  const [kind, setKind] = useState<Kind>("consumption");
  const [itemId, setItemId] = useState<number | null>(null);
  const [description, setDescription] = useState("");
  const [value, setValue] = useState("");
  const [effect, setEffect] = useState<"add" | "reduce">("add");
  const [justification, setJustification] = useState("");
  const [errors, setErrors] = useState<Partial<Record<"description" | "value" | "justification", string>>>({});
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    setKind("consumption");
    setItemId(null);
    setDescription("");
    setValue("");
    setEffect("add");
    setJustification("");
    setErrors({});
    setMessage(null);
  }, [open]);

  const { data: items = [] } = useQuery({ queryKey: ["items", "active"], queryFn: () => listItems(token, true), enabled: open });
  const { requestClose, confirm } = useGuardedClose(description !== "" || value !== "" || justification !== "", onClose);
  const adjustment = kind === "adjustment";

  const pickItem = (id: number | null) => {
    setItemId(id);
    const item = items.find((i) => i.id === id);
    if (item) {
      setDescription(item.name);
      setValue(formatMoney(item.default_price_cents));
    }
  };

  const submit = async () => {
    setMessage(null);
    const next: typeof errors = {};
    const cents = parseMoney(value);
    if (!description.trim()) next.description = "Informe a descrição.";
    if (cents === null || cents <= 0) next.value = "Informe um valor maior que zero, por exemplo R$ 35,00.";
    if (adjustment && !justification.trim()) next.justification = "Informe a justificativa do ajuste.";
    setErrors(next);
    if (Object.keys(next).length > 0 || cents === null) return;

    setBusy(true);
    try {
      if (adjustment) {
        const signed = effect === "reduce" ? -cents : cents;
        await addStayAdjustment(token, stayId, { description: description.trim(), amount_cents: signed, justification: justification.trim() });
        onDone(`Ajuste de ${formatMoney(signed)} lançado na conta.`);
      } else {
        await addStayCharge(token, stayId, { kind, description: description.trim(), amount_cents: cents, item_id: itemId });
        onDone(kind === "consumption" ? `Consumo de ${formatMoney(cents)} lançado na conta.` : `Despesa extra de ${formatMoney(cents)} lançada na conta.`);
      }
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
            <DialogTitle>{adjustment ? "Ajuste da conta" : "Lançar consumo"}</DialogTitle>
            <DialogDescription>
              {adjustment
                ? "O ajuste muda o saldo e fica registrado com a justificativa."
                : "O valor entra na conta da hospedagem. Para corrigir depois, use o estorno."}
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-3">
            <FormField id="ch-kind" label="Tipo de lançamento">
              <select id="ch-kind" className={selectClass} value={kind} onChange={(e) => setKind(e.target.value as Kind)}>
                <option value="consumption">Consumo</option>
                <option value="extra">Despesa extra</option>
                {can("apply_discount") && <option value="adjustment">Ajuste da conta</option>}
              </select>
            </FormField>
            {!adjustment && (
              <FormField id="ch-item" label="Item do catálogo">
                <select id="ch-item" className={selectClass} value={itemId ?? ""} onChange={(e) => pickItem(e.target.value ? Number(e.target.value) : null)}>
                  <option value="">Sem item (lançamento avulso)</option>
                  {items.map((i) => (
                    <option key={i.id} value={i.id}>
                      {`${i.name} — ${formatMoney(i.default_price_cents)}`}
                    </option>
                  ))}
                </select>
              </FormField>
            )}
            {adjustment && (
              <FormField id="ch-effect" label="Efeito no saldo">
                <select id="ch-effect" className={selectClass} value={effect} onChange={(e) => setEffect(e.target.value as "add" | "reduce")}>
                  <option value="add">Cobrar a mais</option>
                  <option value="reduce">Abater do saldo</option>
                </select>
              </FormField>
            )}
            <FormField id="ch-description" label="Descrição" required error={errors.description}>
              <Input id="ch-description" value={description} onChange={(e) => setDescription(e.target.value)} />
            </FormField>
            <FormField id="ch-value" label="Valor" required error={errors.value}>
              <Input id="ch-value" inputMode="decimal" placeholder="R$ 0,00" value={value} onChange={(e) => setValue(e.target.value)} />
            </FormField>
            {adjustment && (
              <FormField id="ch-justification" label="Justificativa" required error={errors.justification}>
                <Textarea id="ch-justification" value={justification} onChange={(e) => setJustification(e.target.value)} />
              </FormField>
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
            <Button type="button" disabled={busy} onClick={submit}>
              {adjustment ? "Lançar ajuste" : "Lançar consumo"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      {confirm}
    </>
  );
}
