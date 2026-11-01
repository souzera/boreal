import type { AlertKind, AlertSeverity } from "@/lib/api";

/** Ícone junto do rótulo: a prioridade nunca depende só de cor. */
export const severityIcon: Record<AlertSeverity, string> = { critical: "error", high: "warning", normal: "info" };

export const severityLabel: Record<AlertSeverity, string> = { critical: "Crítico", high: "Alto", normal: "Normal" };

/** Classes do tema (fundo sutil, texto e borda da semântica correspondente). */
export const severityClass: Record<AlertSeverity, string> = {
  critical: "border-danger bg-danger-subtle text-danger-foreground",
  high: "border-warning bg-warning-subtle text-warning-foreground",
  normal: "border-info bg-info-subtle text-info-foreground",
};

export const kindOptions: { value: AlertKind; label: string }[] = [
  { value: "arrival_today", label: "Chegada hoje" },
  { value: "departure_today", label: "Saída hoje" },
  { value: "arrival_overdue", label: "Chegada atrasada" },
  { value: "deposit_overdue", label: "Sinal vencido" },
  { value: "departure_late", label: "Saída atrasada" },
  { value: "departure_risk", label: "Risco para a próxima reserva" },
  { value: "maintenance_conflict", label: "Reserva afetada por manutenção" },
  { value: "cleaning_urgent", label: "Limpeza urgente" },
  { value: "payable_overdue", label: "Conta vencida" },
  { value: "checkout_balance", label: "Saldo pendente na saída" },
  { value: "closing_pending", label: "Fechamento de caixa pendente" },
  { value: "backup_missing", label: "Sem backup válido" },
  { value: "disk_low", label: "Pouco espaço em disco" },
  { value: "update_failed", label: "Falha na atualização" },
];

/** "3 alertas ativos, 1 crítico" — a contagem em palavras. */
export function alertCountLabel(counts: { critical: number; high?: number; normal?: number; total: number }): string {
  if (counts.total === 0) return "Nenhum alerta ativo";
  const total = `${counts.total} ${counts.total === 1 ? "alerta ativo" : "alertas ativos"}`;
  if (counts.critical === 0) return total;
  return `${total}, ${counts.critical} ${counts.critical === 1 ? "crítico" : "críticos"}`;
}
