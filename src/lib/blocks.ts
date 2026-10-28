import type { BlockStatus, CleaningPriority, MaintenanceStatus } from "@/lib/api";
import { formatDate } from "@/lib/format";

/** Situações com rótulo e símbolo: a cor nunca é o único sinal. */
export const blockStatusLabel: Record<BlockStatus, string> = {
  preventive: "Bloqueio preventivo",
  active: "Bloqueado",
  awaiting_release: "Aguardando liberação",
  released: "Liberado",
};

export const blockStatusIcon: Record<BlockStatus, string> = {
  preventive: "◐",
  active: "✖",
  awaiting_release: "⏸",
  released: "✓",
};

export const maintenanceStatusLabel: Record<MaintenanceStatus, string> = {
  open: "Aberta",
  in_progress: "Em andamento",
  awaiting_resolution: "Aguardando resolução",
  completed: "Concluída",
};

export const maintenanceStatusIcon: Record<MaintenanceStatus, string> = {
  open: "○",
  in_progress: "◑",
  awaiting_resolution: "⏸",
  completed: "✓",
};

export const orderPriorityLabel: Record<CleaningPriority, string> = {
  urgent: "Urgente",
  high: "Alta",
  normal: "Normal",
  low: "Baixa",
};

/** `10/11/2026 a 13/11/2026` ou `a partir de 10/11/2026, até ser liberado`. */
export function periodText(start: string, end: string | null): string {
  return end ? `${formatDate(start)} a ${formatDate(end)}` : `a partir de ${formatDate(start)}, até ser liberado`;
}
