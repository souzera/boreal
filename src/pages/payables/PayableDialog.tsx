import { useEffect, useState } from "react";

import { FormField, RequiredLegend } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { useGuardedClose } from "@/hooks/useGuardedClose";
import { createPayable, type ApiError, type ExpenseCategory, type SelectableMethod } from "@/lib/api";
import { parseMoney, toLocalIsoDate } from "@/lib/format";
import { methodLabel, selectableMethods } from "@/lib/payment";
import { useSessionStore } from "@/stores/session";

interface Props {
  open: boolean;
  categories: ExpenseCategory[];
  onClose: () => void;
  onDone: (message: string) => void;
}

type Field = "supplier" | "description" | "amount" | "competence" | "due" | "category" | "method";

const selectClass = "h-8 rounded-lg border border-input bg-transparent px-2 text-sm";

/** Cadastro de uma conta a pagar, ou de uma despesa já paga à vista. */
export default function PayableDialog({ open, categories, onClose, onDone }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const [supplier, setSupplier] = useState("");
  const [description, setDescription] = useState("");
  const [amount, setAmount] = useState("");
  const [competence, setCompetence] = useState("");
  const [due, setDue] = useState("");
  const [category, setCategory] = useState("");
  const [notes, setNotes] = useState("");
  const [paidNow, setPaidNow] = useState(false);
  const [method, setMethod] = useState<SelectableMethod | "">("");
  const [errors, setErrors] = useState<Partial<Record<Field, string>>>({});
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    setSupplier("");
    setDescription("");
    setAmount("");
    setCompetence(toLocalIsoDate(new Date()));
    setDue("");
    setCategory("");
    setNotes("");
    setPaidNow(false);
    setMethod("");
    setErrors({});
    setMessage(null);
  }, [open]);

  const dirty = supplier !== "" || description !== "" || amount !== "" || due !== "" || notes !== "";
  const { requestClose, confirm } = useGuardedClose(dirty, onClose);

  const submit = async () => {
    setMessage(null);
    const cents = parseMoney(amount);
    const next: Partial<Record<Field, string>> = {};
    if (supplier.trim() === "") next.supplier = "Informe o fornecedor.";
    if (description.trim() === "") next.description = "Informe a descrição.";
    if (cents === null || cents <= 0) next.amount = "Informe um valor maior que zero, por exemplo R$ 500,00.";
    if (competence === "") next.competence = "Informe a data da despesa.";
    if (due === "") next.due = "Informe o vencimento.";
    else if (competence !== "" && due < competence) next.due = "O vencimento não pode ser antes da data da despesa.";
    if (category === "") next.category = "Escolha a categoria.";
    if (paidNow && method === "") next.method = "Escolha a forma de pagamento.";
    setErrors(next);
    if (Object.keys(next).length > 0 || cents === null) return;
    setBusy(true);
    try {
      await createPayable(token, {
        supplier: supplier.trim(),
        description: description.trim(),
        amount_cents: cents,
        competence_date: competence,
        due_date: due,
        category_id: Number(category),
        notes: notes.trim(),
        ...(paidNow && method !== "" ? { paid_now: { method } } : {}),
      });
      onDone(paidNow ? "Despesa registrada como paga e salva neste computador." : "Conta a pagar registrada e salva neste computador.");
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
            <DialogTitle>Registrar conta a pagar</DialogTitle>
          </DialogHeader>
          <div className="grid gap-3">
            <FormField id="pb-supplier" label="Fornecedor" required error={errors.supplier}>
              <Input id="pb-supplier" value={supplier} onChange={(e) => setSupplier(e.target.value)} />
            </FormField>
            <FormField id="pb-description" label="Descrição" required error={errors.description}>
              <Input id="pb-description" value={description} onChange={(e) => setDescription(e.target.value)} />
            </FormField>
            <FormField id="pb-amount" label="Valor da conta" required error={errors.amount}>
              <Input id="pb-amount" inputMode="decimal" placeholder="R$ 0,00" value={amount} onChange={(e) => setAmount(e.target.value)} />
            </FormField>
            <div className="grid gap-3 sm:grid-cols-2">
              <FormField id="pb-competence" label="Data da despesa" required error={errors.competence}>
                <Input id="pb-competence" type="date" value={competence} onChange={(e) => setCompetence(e.target.value)} />
              </FormField>
              <FormField id="pb-due" label="Vencimento" required error={errors.due}>
                <Input id="pb-due" type="date" value={due} onChange={(e) => setDue(e.target.value)} />
              </FormField>
            </div>
            <FormField id="pb-category" label="Categoria" required error={errors.category}>
              <select id="pb-category" className={selectClass} value={category} onChange={(e) => setCategory(e.target.value)}>
                <option value="">Escolha…</option>
                {categories
                  .filter((c) => c.active)
                  .map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
              </select>
            </FormField>
            <FormField id="pb-notes" label="Observações">
              <Input id="pb-notes" value={notes} onChange={(e) => setNotes(e.target.value)} />
            </FormField>
            <label className="flex items-center gap-2 text-sm">
              <input type="checkbox" checked={paidNow} onChange={(e) => setPaidNow(e.target.checked)} />
              Já paga à vista
            </label>
            {paidNow && (
              <FormField id="pb-method" label="Forma de pagamento" required error={errors.method}>
                <select
                  id="pb-method"
                  className={selectClass}
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
            )}
            {message && (
              <p role="alert" className="text-sm text-destructive">
                {message}
              </p>
            )}
            <RequiredLegend />
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={requestClose}>
              Cancelar
            </Button>
            <Button type="button" disabled={busy} onClick={() => void submit()}>
              Registrar conta a pagar
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      {confirm}
    </>
  );
}
