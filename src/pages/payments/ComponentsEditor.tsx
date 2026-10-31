import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { formatMoney } from "@/lib/format";
import { emptyRow, methodLabel, selectableMethods, sumRows, type SplitRow } from "@/lib/payment";

interface Props {
  /** Total informado, em centavos, ou `null` se ainda inválido. */
  totalCents: number | null;
  rows: SplitRow[];
  onChange: (rows: SplitRow[]) => void;
  /** Mensagem de erro da forma de pagamento (exibida abaixo dos campos). */
  error?: string;
}

const selectClass = "h-8 rounded-lg border border-input bg-transparent px-2 text-sm";

/** Escolha da forma de pagamento, com divisão entre várias formas e a conferência da soma. */
export default function ComponentsEditor({ totalCents, rows, onChange, error }: Props) {
  const split = rows.length > 1;
  const update = (index: number, patch: Partial<SplitRow>) =>
    onChange(rows.map((row, i) => (i === index ? { ...row, ...patch } : row)));

  const sum = sumRows(rows);
  const remaining = totalCents === null ? null : totalCents - sum;

  return (
    <div className="grid gap-2">
      {rows.map((row, index) => (
        <div key={index} className="flex flex-wrap items-end gap-2">
          <div className="grid gap-1.5">
            <Label htmlFor={`method-${index}`}>
              {split ? `Forma de pagamento ${index + 1}` : "Forma de pagamento"}
              <span aria-hidden="true"> *</span>
            </Label>
            <select
              id={`method-${index}`}
              aria-label={split ? `Forma de pagamento ${index + 1}` : "Forma de pagamento"}
              className={selectClass}
              value={row.method}
              onChange={(e) => update(index, { method: e.target.value as SplitRow["method"] })}
            >
              <option value="">Escolha…</option>
              {selectableMethods.map((m) => (
                <option key={m} value={m}>
                  {methodLabel[m]}
                </option>
              ))}
            </select>
          </div>
          {split && (
            <div className="grid gap-1.5">
              <Label htmlFor={`amount-${index}`}>{`Valor da forma ${index + 1}`}</Label>
              <Input
                id={`amount-${index}`}
                aria-label={`Valor da forma ${index + 1}`}
                inputMode="decimal"
                placeholder="R$ 0,00"
                value={row.amount}
                onChange={(e) => update(index, { amount: e.target.value })}
              />
            </div>
          )}
          {split && (
            <Button type="button" variant="outline" size="sm" onClick={() => onChange(rows.filter((_, i) => i !== index))}>
              Remover forma
            </Button>
          )}
        </div>
      ))}
      {error && (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      )}
      {split && remaining !== null && totalCents !== null && (
        <p role="status" className="text-sm font-medium">
          {`Soma ${formatMoney(sum)} de ${formatMoney(totalCents)} · `}
          {remaining === 0 ? "✓ Confere" : remaining > 0 ? `⚠ Falta ${formatMoney(remaining)}` : `⚠ Sobra ${formatMoney(-remaining)}`}
        </p>
      )}
      {rows.length < selectableMethods.length && (
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="w-fit"
          onClick={() => onChange([...rows, emptyRow()])}
        >
          Dividir em outra forma
        </Button>
      )}
    </div>
  );
}
