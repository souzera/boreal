import type { AppliedRule, ConflictInfo, ReservationStatus } from "@/lib/api";

export const statusLabel: Record<ReservationStatus, string> = {
  pending: "Pendente",
  confirmed: "Confirmada",
  checked_in: "Hospedado",
  completed: "Concluída",
  cancelled: "Cancelada",
  no_show: "No-show",
};

/** Ícone curto junto do texto: a situação nunca depende só de cor. */
export const statusIcon: Record<ReservationStatus, string> = {
  pending: "◔",
  confirmed: "✔",
  checked_in: "●",
  completed: "■",
  cancelled: "✖",
  no_show: "⚠",
};

export const ruleLabel: Record<AppliedRule, string> = {
  date_override: "Exceção de data",
  holiday: "Feriado",
  season: "Temporada",
  weekday: "Dia da semana",
};

/** Estados em que a reserva ainda pode ser alterada, cancelada ou confirmada. */
export const isOpen = (status: ReservationStatus) => status === "pending" || status === "confirmed";

export function conflictText(conflict: ConflictInfo): string {
  return `${conflict.code} (${conflict.guest_name})`;
}
