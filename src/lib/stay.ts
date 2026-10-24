import type { ChargeKind, PaymentMethod, StayStatus } from "@/lib/api";
import { formatMoney } from "@/lib/format";

export const stayStatusLabel: Record<StayStatus, string> = {
  in_house: "Hospedado",
  checked_out: "Encerrada",
};

/** Ícone curto junto do texto: a situação nunca depende só de cor. */
export const stayStatusIcon: Record<StayStatus, string> = {
  in_house: "●",
  checked_out: "■",
};

export const chargeKindLabel: Record<ChargeKind, string> = {
  night: "Diária",
  consumption: "Consumo",
  extra: "Despesa extra",
  discount: "Desconto",
  adjustment: "Ajuste",
  late_fee: "Saída tardia",
  reversal: "Estorno",
};

export const paymentMethodLabel: Record<PaymentMethod, string> = {
  cash: "Dinheiro",
  pix: "Pix",
  debit: "Cartão de débito",
  credit: "Cartão de crédito",
  transfer: "Transferência",
  prior_deposit: "Sinal recebido antes do check-in",
};

/** Formas de pagamento que a recepção pode escolher (o sinal anterior só aparece na conta). */
export const selectableMethods: Exclude<PaymentMethod, "prior_deposit">[] = ["cash", "pix", "debit", "credit", "transfer"];

/** Situação da conta em palavras: nunca só um número com sinal. */
export function balanceLabel(balanceCents: number): string {
  if (balanceCents > 0) return `Saldo a receber ${formatMoney(balanceCents)}`;
  if (balanceCents < 0) return `Crédito do hóspede ${formatMoney(-balanceCents)}`;
  return "Quitado";
}
