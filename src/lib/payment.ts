import type { ComponentInput, ComponentLine, PaymentMethod, SelectableMethod } from "@/lib/api";
import { formatMoney, parseMoney } from "@/lib/format";

export const methodLabel: Record<PaymentMethod, string> = {
  cash: "Dinheiro",
  pix: "Pix",
  debit: "Cartão de débito",
  credit: "Cartão de crédito",
  transfer: "Transferência bancária",
  unspecified: "Sem modalidade",
};

/** Formas de pagamento que a recepção escolhe; débito e crédito são sempre separados. */
export const selectableMethods: SelectableMethod[] = ["cash", "pix", "debit", "credit", "transfer"];

/** Uma linha da divisão: a forma e o valor digitado (texto, como o usuário escreveu). */
export interface SplitRow {
  method: SelectableMethod | "";
  amount: string;
}

export const emptyRow = (): SplitRow => ({ method: "", amount: "" });

/** "Pix R$ 60,00 + Dinheiro R$ 40,00" */
export function describeComponents(components: ComponentLine[]): string {
  return components.map((c) => `${methodLabel[c.method]} ${formatMoney(c.amount_cents)}`).join(" + ");
}

/** Soma dos valores válidos digitados nas linhas. */
export function sumRows(rows: SplitRow[]): number {
  return rows.reduce((total, row) => total + (parseMoney(row.amount) ?? 0), 0);
}

export interface SplitCheck {
  /** Componentes prontos para enviar; `null` enquanto houver problema. */
  components: ComponentInput[] | null;
  sum: number;
  /** Quanto falta (positivo) ou sobra (negativo) para fechar o total. */
  remaining: number;
  error: string | null;
}

/**
 * Confere a divisão antes de enviar. Com uma forma só, o valor é o total informado; com várias,
 * cada linha traz o seu valor e a soma precisa fechar o total (o backend confere de novo).
 */
export function checkSplit(totalCents: number | null, rows: SplitRow[]): SplitCheck {
  const fail = (error: string, sum = 0, remaining = 0): SplitCheck => ({ components: null, sum, remaining, error });
  if (totalCents === null || totalCents <= 0) return fail("Informe um valor maior que zero, por exemplo R$ 100,00.");
  if (rows.length === 1) {
    const [only] = rows;
    if (only.method === "") return fail("Escolha a forma de pagamento.", totalCents);
    return { components: [{ method: only.method, amount_cents: totalCents }], sum: totalCents, remaining: 0, error: null };
  }
  const sum = sumRows(rows);
  const remaining = totalCents - sum;
  if (rows.some((r) => r.method === "")) return fail("Escolha a forma de pagamento.", sum, remaining);
  const amounts = rows.map((r) => parseMoney(r.amount));
  if (amounts.some((a) => a === null || a <= 0)) return fail("Informe o valor de cada forma de pagamento.", sum, remaining);
  const methods = rows.map((r) => r.method);
  if (new Set(methods).size !== methods.length) return fail("Cada forma de pagamento só pode aparecer uma vez.", sum, remaining);
  if (remaining !== 0) return fail(`A soma das formas de pagamento precisa ser ${formatMoney(totalCents)}.`, sum, remaining);
  return {
    components: rows.map((r, i) => ({ method: r.method as SelectableMethod, amount_cents: amounts[i] as number })),
    sum,
    remaining: 0,
    error: null,
  };
}
