import type { ReportCell, ReportKind } from "@/lib/api";
import { formatMoney } from "@/lib/format";

export type FilterKey = "status" | "room" | "method" | "category" | "text";

export interface ReportInfo {
  kind: ReportKind;
  label: string;
  /** Relatório financeiro detalhado: só o administrador (espelha o backend). */
  financial: boolean;
  description: string;
  /** Mostra o filtro de período? "Saldos pendentes" é a situação de agora. */
  period: boolean;
  filters: FilterKey[];
}

/** Os dez relatórios do §12.3, na ordem em que aparecem na tela. */
export const REPORTS: ReportInfo[] = [
  { kind: "occupancy", label: "Ocupação", financial: false, description: "Quartos ocupados e livres em cada dia.", period: true, filters: ["room"] },
  { kind: "reservations", label: "Reservas", financial: false, description: "Reservas com chegada no período.", period: true, filters: ["status", "room", "text"] },
  { kind: "stays", label: "Hospedagens", financial: false, description: "Hospedagens com check-in no período.", period: true, filters: ["status", "room", "text"] },
  { kind: "cancellations", label: "Cancelamentos e no-shows", financial: false, description: "Reservas canceladas ou marcadas como no-show no período.", period: true, filters: ["status", "room", "text"] },
  { kind: "receipts", label: "Recebimentos e devoluções", financial: true, description: "Cada recebimento e devolução, com débito e crédito separados.", period: true, filters: ["method", "room", "text"] },
  { kind: "balances", label: "Saldos pendentes", financial: true, description: "Contas em andamento com saldo a receber ou crédito, na situação de agora.", period: false, filters: ["room", "text"] },
  { kind: "guest_expenses", label: "Despesas extras dos hóspedes", financial: true, description: "Consumos e despesas extras lançados no período, com estornos.", period: true, filters: ["room", "text"] },
  { kind: "property_expenses", label: "Despesas da pousada", financial: true, description: "Despesas por data de competência; mostra também o que foi pago no período.", period: true, filters: ["status", "category", "text"] },
  { kind: "payables", label: "Contas a pagar", financial: true, description: "Contas por data de vencimento, com o que foi pago e o que falta.", period: true, filters: ["status", "category", "text"] },
  { kind: "closings", label: "Fechamentos de caixa", financial: true, description: "Esperado, contado e diferença de cada dia fechado.", period: true, filters: ["method"] },
];

export const reportInfo = (kind: string | null | undefined): ReportInfo | undefined => REPORTS.find((r) => r.kind === kind);

export const STATUS_OPTIONS: Partial<Record<ReportKind, { value: string; label: string }[]>> = {
  reservations: [
    { value: "pending", label: "Pendente" },
    { value: "confirmed", label: "Confirmada" },
    { value: "checked_in", label: "Hospedado" },
    { value: "completed", label: "Concluída" },
    { value: "cancelled", label: "Cancelada" },
    { value: "no_show", label: "No-show" },
  ],
  stays: [
    { value: "in_house", label: "Hospedado" },
    { value: "checked_out", label: "Check-out feito" },
  ],
  cancellations: [
    { value: "cancelled", label: "Cancelamento" },
    { value: "no_show", label: "No-show" },
  ],
  property_expenses: [
    { value: "pending", label: "Pendente" },
    { value: "partial", label: "Parcial" },
    { value: "paid", label: "Paga" },
    { value: "overdue", label: "Vencida" },
  ],
  payables: [
    { value: "pending", label: "Pendente" },
    { value: "partial", label: "Parcial" },
    { value: "paid", label: "Paga" },
    { value: "overdue", label: "Vencida" },
  ],
};

export const METHOD_OPTIONS = [
  { value: "cash", label: "Dinheiro" },
  { value: "pix", label: "Pix" },
  { value: "debit", label: "Cartão de débito" },
  { value: "credit", label: "Cartão de crédito" },
  { value: "transfer", label: "Transferência bancária" },
];

const MAX_DAYS = 366;

/** Texto de erro do período (ou `null` se está válido). Mesmas regras do backend. */
export function periodError(from: string, to: string): string | null {
  if (!from || !to) return "Informe as duas datas do período.";
  if (to < from) return "A data final não pode ser antes da inicial.";
  const days = Math.round((Date.parse(to) - Date.parse(from)) / 86_400_000) + 1;
  if (days > MAX_DAYS) return "O período pode ter no máximo 366 dias.";
  return null;
}

export function percentText(basisPoints: number): string {
  const tenths = Math.round(Math.abs(basisPoints) / 10);
  return `${basisPoints < 0 ? "-" : ""}${Math.floor(tenths / 10)},${tenths % 10}%`;
}

/** Texto de uma célula como aparece na tela (igual ao do PDF). */
export function cellText(cell: ReportCell): string {
  switch (cell.kind) {
    case "money":
      return formatMoney(cell.value);
    case "percent":
      return percentText(cell.value);
    case "int":
      return String(cell.value);
    case "text":
      return cell.value;
    default:
      return "";
  }
}

/** Nome sugerido no "Salvar como": `relatorio-receipts-2026-10-21.csv`. */
export const suggestedFileName = (kind: ReportKind, extension: "csv" | "pdf", isoDate: string) => `relatorio-${kind.replace(/_/g, "-")}-${isoDate}.${extension}`;
