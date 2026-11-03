import type { CleaningPriority, RoomStateKey } from "@/lib/api";

/** Rótulos dos seis estados do quarto (spec §7). */
export const ROOM_STATE_LABEL: Record<RoomStateKey, string> = {
  occupied: "Ocupado",
  awaiting_cleaning: "Aguardando limpeza",
  cleaning: "Em limpeza",
  clean_waiting_interval: "Limpo — aguardando intervalo",
  available: "Disponível",
  blocked: "Em manutenção ou bloqueado",
};

/** Cada estado combina cor (tokens do tema), ícone e rótulo: a cor nunca é o único sinal. */
export const ROOM_STATE_STYLE: Record<RoomStateKey, { icon: string; className: string }> = {
  occupied: {
    icon: "bed",
    className: "bg-state-occupied-bg text-state-occupied-fg border-state-occupied-border",
  },
  awaiting_cleaning: {
    icon: "cleaning_services",
    className: "bg-state-awaiting-cleaning-bg text-state-awaiting-cleaning-fg border-state-awaiting-cleaning-border",
  },
  cleaning: {
    icon: "autorenew",
    className: "bg-state-cleaning-bg text-state-cleaning-fg border-state-cleaning-border",
  },
  clean_waiting_interval: {
    icon: "hourglass_top",
    className: "bg-state-clean-waiting-interval-bg text-state-clean-waiting-interval-fg border-state-clean-waiting-interval-border",
  },
  available: {
    icon: "check_circle",
    className: "bg-state-available-bg text-state-available-fg border-state-available-border",
  },
  blocked: {
    icon: "block",
    className: "bg-state-blocked-bg text-state-blocked-fg border-state-blocked-border",
  },
};

export const ROOM_STATE_ORDER: RoomStateKey[] = [
  "occupied",
  "awaiting_cleaning",
  "cleaning",
  "clean_waiting_interval",
  "available",
  "blocked",
];

export const PRIORITY_LABEL: Record<CleaningPriority, string> = {
  urgent: "Urgente",
  high: "Alta",
  normal: "Normal",
  low: "Baixa",
};

export const PRIORITY_ICON: Record<CleaningPriority, string> = {
  urgent: "keyboard_double_arrow_up",
  high: "keyboard_arrow_up",
  normal: "remove",
  low: "keyboard_arrow_down",
};

/** `AAAA-MM-DDTHH:MM` (horário local) em `DD/MM/AAAA HH:MM`. */
export function formatLocalDateTime(value: string): string {
  const [date, time] = value.split("T");
  const [y, m, d] = date.split("-");
  return `${d}/${m}/${y} ${time ?? ""}`.trim();
}
