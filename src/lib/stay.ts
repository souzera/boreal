import type { ChargeKind, StayStatus } from "@/lib/api";
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

/** Situação da conta em palavras: nunca só um número com sinal. */
export function balanceLabel(balanceCents: number): string {
  if (balanceCents > 0) return `Saldo a receber ${formatMoney(balanceCents)}`;
  if (balanceCents < 0) return `Crédito do hóspede ${formatMoney(-balanceCents)}`;
  return "Quitado";
}
