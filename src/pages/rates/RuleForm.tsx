import { useEffect, useState } from "react";

import { FormField, RequiredLegend } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { useGuardedClose } from "@/hooks/useGuardedClose";
import type { ApiError, RateRule, RateRuleInput, RateRuleType } from "@/lib/api";
import { formatMoney, parseMoney } from "@/lib/format";

export const weekdayNames = [
  "Domingo",
  "Segunda-feira",
  "Terça-feira",
  "Quarta-feira",
  "Quinta-feira",
  "Sexta-feira",
  "Sábado",
];

const titles: Record<RateRuleType, string> = {
  weekday: "Tarifa por dia da semana",
  season: "Temporada",
  holiday: "Preço nos feriados cadastrados",
  date_override: "Exceção de data",
};

interface Props {
  open: boolean;
  categoryId: number;
  type: RateRuleType;
  /** Regra em edição; `null` cria. */
  rule: RateRule | null;
  /** Dia da semana pré-escolhido ao definir uma tarifa nova. */
  weekday?: number;
  onClose: () => void;
  onSubmit: (input: RateRuleInput) => Promise<void>;
}

interface Fields {
  value: string;
  start: string;
  end: string;
  date: string;
  label: string;
}

const blank: Fields = { value: "", start: "", end: "", date: "", label: "" };

export default function RuleForm({ open, categoryId, type, rule, weekday, onClose, onSubmit }: Props) {
  const [fields, setFields] = useState<Fields>(blank);
  const [initial, setInitial] = useState<Fields>(blank);
  const [errors, setErrors] = useState<Partial<Record<keyof Fields, string>>>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    const start: Fields = rule
      ? {
          value: formatMoney(rule.amount_cents).replace("R$ ", ""),
          start: rule.start_date ?? "",
          end: rule.end_date ?? "",
          date: rule.date ?? "",
          label: rule.label,
        }
      : blank;
    setFields(start);
    setInitial(start);
    setErrors({});
    setFormError(null);
  }, [open, rule]);

  const dirty = (Object.keys(fields) as (keyof Fields)[]).some((k) => fields[k] !== initial[k]);
  const { requestClose, confirm } = useGuardedClose(dirty, onClose);
  const set = (key: keyof Fields) => (e: { target: { value: string } }) =>
    setFields((f) => ({ ...f, [key]: e.target.value }));

  const submit = async (e: { preventDefault: () => void }) => {
    e.preventDefault();
    const next: Partial<Record<keyof Fields, string>> = {};
    const cents = parseMoney(fields.value);
    if (cents === null || cents <= 0) next.value = "Informe um valor, por exemplo 180,00.";
    if (type === "season") {
      if (!fields.start) next.start = "Informe o início da temporada.";
      if (!fields.end) next.end = "Informe o fim da temporada.";
      if (fields.start && fields.end && fields.start > fields.end) next.end = "O fim deve ser depois do início.";
    }
    if (type === "date_override" && !fields.date) next.date = "Informe a data.";
    setErrors(next);
    if (Object.keys(next).length > 0 || cents === null) return;

    setSaving(true);
    setFormError(null);
    try {
      await onSubmit({
        category_id: categoryId,
        rule_type: type,
        weekday: type === "weekday" ? (rule?.weekday ?? weekday ?? 0) : null,
        start_date: type === "season" ? fields.start : null,
        end_date: type === "season" ? fields.end : null,
        date: type === "date_override" ? fields.date : null,
        amount_cents: cents,
        label: fields.label.trim(),
      });
    } catch (err) {
      setFormError((err as ApiError).message);
    } finally {
      setSaving(false);
    }
  };

  const dayName = type === "weekday" ? weekdayNames[rule?.weekday ?? weekday ?? 0] : null;

  return (
    <>
      <Dialog open={open} onOpenChange={(next) => !next && requestClose()}>
        <DialogContent showCloseButton={false}>
          <DialogHeader>
            <DialogTitle>{dayName ? `${titles[type]}: ${dayName}` : titles[type]}</DialogTitle>
            <DialogDescription>
              {type === "holiday"
                ? "Vale em todas as datas do calendário de feriados."
                : "O valor é a diária por noite, para toda a categoria."}
            </DialogDescription>
          </DialogHeader>
          <form onSubmit={submit} noValidate className="grid gap-4">
            {type === "season" && (
              <>
                <FormField id="rule-start" label="Início" required error={errors.start}>
                  <Input id="rule-start" type="date" value={fields.start} onChange={set("start")} />
                </FormField>
                <FormField id="rule-end" label="Fim" required error={errors.end}>
                  <Input id="rule-end" type="date" value={fields.end} onChange={set("end")} />
                </FormField>
              </>
            )}
            {type === "date_override" && (
              <FormField id="rule-date" label="Data" required error={errors.date}>
                <Input id="rule-date" type="date" value={fields.date} onChange={set("date")} />
              </FormField>
            )}
            {(type === "season" || type === "date_override") && (
              <FormField id="rule-label" label="Descrição">
                <Input id="rule-label" value={fields.label} onChange={set("label")} />
              </FormField>
            )}
            <FormField id="rule-value" label="Valor da diária (R$)" required error={errors.value}>
              <Input id="rule-value" inputMode="decimal" placeholder="180,00" value={fields.value} onChange={set("value")} />
            </FormField>
            {formError && (
              <p role="alert" className="text-sm text-destructive">
                {formError}
              </p>
            )}
            <RequiredLegend />
            <DialogFooter>
              <Button type="button" variant="outline" onClick={requestClose}>
                Cancelar
              </Button>
              <Button type="submit" disabled={saving}>
                Salvar tarifa
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
      {confirm}
    </>
  );
}
