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

/** Cada estado combina cor, símbolo e rótulo: a cor nunca é o único sinal. */
export const ROOM_STATE_STYLE: Record<RoomStateKey, { icon: string; className: string }> = {
  occupied: { icon: "●", className: "bg-blue-100 text-blue-900 border-blue-300" },
  awaiting_cleaning: { icon: "◐", className: "bg-amber-100 text-amber-900 border-amber-300" },
  cleaning: { icon: "◑", className: "bg-violet-100 text-violet-900 border-violet-300" },
  clean_waiting_interval: { icon: "◔", className: "bg-cyan-100 text-cyan-900 border-cyan-300" },
  available: { icon: "✓", className: "bg-green-100 text-green-900 border-green-300" },
  blocked: { icon: "✖", className: "bg-red-100 text-red-900 border-red-300" },
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
  urgent: "▲▲",
  high: "▲",
  normal: "●",
  low: "▽",
};

/** `AAAA-MM-DDTHH:MM` (horário local) em `DD/MM/AAAA HH:MM`. */
export function formatLocalDateTime(value: string): string {
  const [date, time] = value.split("T");
  const [y, m, d] = date.split("-");
  return `${d}/${m}/${y} ${time ?? ""}`.trim();
}
