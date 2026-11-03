import { ConfirmDialog } from "@/components/ConfirmDialog";
import { formatDate, formatMoney } from "@/lib/format";

export interface Difference {
  label: string;
  cents: number;
}

interface Props {
  open: boolean;
  date: string;
  differences: Difference[];
  onConfirm: () => void;
  onCancel: () => void;
}

/** Última conferência antes de gravar o fechamento do dia (depois de gravado, ele não muda). */
export default function ClosingDialog({ open, date, differences, onConfirm, onCancel }: Props) {
  const summary =
    differences.length === 0
      ? "Nenhuma diferença encontrada."
      : `Diferenças que serão registradas: ${differences
          .map((d) => `${d.label}: ${d.cents < 0 ? "falta" : "sobra"} ${formatMoney(Math.abs(d.cents))}`)
          .join("; ")}.`;
  return (
    <ConfirmDialog
      open={open}
      title={`Concluir o fechamento de ${formatDate(date)}?`}
      description={`${summary} Depois de concluído, o fechamento deste dia não pode ser alterado e os recebimentos não são modificados.`}
      confirmLabel="Concluir fechamento"
      cancelLabel="Voltar e conferir"
      onConfirm={onConfirm}
      onCancel={onCancel}
    />
  );
}
