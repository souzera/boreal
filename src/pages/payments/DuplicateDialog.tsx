import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { DuplicateInfo } from "@/lib/api";
import { formatInstant, formatMoney } from "@/lib/format";
import { methodLabel } from "@/lib/payment";
import Icon from "@/components/Icon";

interface Props {
  info: DuplicateInfo;
  canRelease: boolean;
  justification: string;
  error: string | null;
  busy: boolean;
  onJustification: (value: string) => void;
  onRelease: () => void;
  onBack: () => void;
}

/** Aviso de possível recebimento duplicado dentro do diálogo de pagamento; só quem tem permissão libera. */
export default function DuplicateDialog({ info, canRelease, justification, error, busy, onJustification, onRelease, onBack }: Props) {
  const methods = info.methods.map((m) => methodLabel[m]).join(" + ");
  return (
    <div className="grid gap-3">
      <p role="alert" className="rounded-lg border border-destructive/40 bg-destructive/5 p-3 text-sm">
        <Icon name="warning" size="xs" className="mr-1 align-middle" />{`Possível recebimento duplicado: já existe o ${info.code}, de ${formatMoney(info.total_cents)} em ${methods}, registrado em ${formatInstant(info.received_at)}. Confira antes de registrar de novo.`}
      </p>
      {canRelease ? (
        <FormField id="dup-justification" label="Justificativa para liberar" required error={error ?? undefined}>
          <Input id="dup-justification" value={justification} onChange={(e) => onJustification(e.target.value)} />
        </FormField>
      ) : (
        <p className="text-sm text-muted-foreground">
          Somente quem tem a permissão Liberar recebimento duplicado pode registrar este pagamento. Chame um administrador ou corrija os dados.
        </p>
      )}
      <div className="flex flex-wrap justify-end gap-2">
        <Button type="button" variant="outline" onClick={onBack}>
          Voltar e corrigir
        </Button>
        {canRelease && (
          <Button type="button" disabled={busy} onClick={onRelease}>
            Liberar recebimento duplicado
          </Button>
        )}
      </div>
    </div>
  );
}
