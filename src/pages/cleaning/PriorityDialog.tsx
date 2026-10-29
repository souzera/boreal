import { useEffect, useState } from "react";

import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Textarea } from "@/components/ui/textarea";
import { useGuardedClose } from "@/hooks/useGuardedClose";
import { setCleaningPriority, type ApiError, type CleaningItem, type CleaningPriority } from "@/lib/api";
import { PRIORITY_LABEL } from "@/lib/roomState";
import { useSessionStore } from "@/stores/session";

interface Props {
  open: boolean;
  item: CleaningItem | null;
  onClose: () => void;
  onDone: (message: string) => void;
}

const OPTIONS: CleaningPriority[] = ["urgent", "high", "normal", "low"];

/** Ajusta a prioridade sugerida da limpeza; a justificativa é obrigatória e fica na auditoria. */
export default function PriorityDialog({ open, item, onClose, onDone }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const [priority, setPriority] = useState<CleaningPriority>("normal");
  const [justification, setJustification] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open || !item) return;
    setPriority(item.priority);
    setJustification("");
    setError(null);
    setMessage(null);
  }, [open, item]);

  const { requestClose, confirm } = useGuardedClose(justification !== "", onClose);
  if (!item) return null;

  const submit = async (value: CleaningPriority | null) => {
    setMessage(null);
    if (!justification.trim()) {
      setError("Informe a justificativa do ajuste.");
      return;
    }
    setError(null);
    setBusy(true);
    try {
      await setCleaningPriority(token, item.task.id, value, justification.trim(), item.task.version);
      onDone(
        value
          ? `Prioridade do quarto ${item.task.room_code} ajustada para ${PRIORITY_LABEL[value]}.`
          : `Prioridade do quarto ${item.task.room_code} voltou à sugerida.`,
      );
    } catch (e) {
      setMessage((e as ApiError).message);
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <Dialog open={open} onOpenChange={(next) => !next && requestClose()}>
        <DialogContent showCloseButton={false}>
          <DialogHeader>
            <DialogTitle>{`Ajustar prioridade do quarto ${item.task.room_code}`}</DialogTitle>
            <DialogDescription>
              {`A sugestão do sistema é ${PRIORITY_LABEL[item.suggested_priority]}. O ajuste e a justificativa ficam registrados.`}
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-3">
            <FormField id="cp-priority" label="Nova prioridade">
              <select
                id="cp-priority"
                className="h-8 rounded-lg border border-input bg-transparent px-2 text-sm"
                value={priority}
                onChange={(e) => setPriority(e.target.value as CleaningPriority)}
              >
                {OPTIONS.map((p) => (
                  <option key={p} value={p}>
                    {PRIORITY_LABEL[p]}
                  </option>
                ))}
              </select>
            </FormField>
            <FormField id="cp-justification" label="Justificativa do ajuste" required error={error ?? undefined}>
              <Textarea id="cp-justification" value={justification} onChange={(e) => setJustification(e.target.value)} />
            </FormField>
            {message && (
              <p role="alert" className="text-sm text-destructive">
                {message}
              </p>
            )}
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={requestClose}>
              Cancelar
            </Button>
            {item.task.priority_override && (
              <Button type="button" variant="outline" disabled={busy} onClick={() => submit(null)}>
                Voltar à sugerida
              </Button>
            )}
            <Button type="button" disabled={busy} onClick={() => submit(priority)}>
              Salvar prioridade
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      {confirm}
    </>
  );
}
