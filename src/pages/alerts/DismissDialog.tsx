import { useEffect, useState } from "react";

import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { useGuardedClose } from "@/hooks/useGuardedClose";
import { dismissAlert, type AlertView, type ApiError } from "@/lib/api";
import { useSessionStore } from "@/stores/session";

interface Props {
  open: boolean;
  alert: AlertView;
  onClose: () => void;
  onDone: (message: string) => void;
}

/** Dispensa um alerta não crítico, com justificativa (fica no histórico e na auditoria). */
export default function DismissDialog({ open, alert, onClose, onDone }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const [note, setNote] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    setNote("");
    setError(null);
    setMessage(null);
  }, [open]);

  const { requestClose, confirm } = useGuardedClose(note !== "", onClose);

  const submit = async () => {
    setMessage(null);
    if (note.trim() === "") {
      setError("Explique por que o alerta está sendo dispensado.");
      return;
    }
    setError(null);
    setBusy(true);
    try {
      await dismissAlert(token, alert.id, note.trim(), alert.version);
      onDone("Alerta dispensado e salvo neste computador.");
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
            <DialogTitle>Dispensar alerta</DialogTitle>
            <DialogDescription>
              {`${alert.title}. O alerta sai da lista de ativos e fica no histórico com a sua justificativa. Se o problema deixar de existir e voltar depois, um novo alerta aparece.`}
            </DialogDescription>
          </DialogHeader>
          <FormField id="dismiss-note" label="Justificativa" required error={error ?? undefined}>
            <Input id="dismiss-note" value={note} onChange={(e) => setNote(e.target.value)} />
          </FormField>
          {message && (
            <p role="alert" className="text-sm text-destructive">
              {message}
            </p>
          )}
          <DialogFooter>
            <Button type="button" variant="outline" onClick={requestClose}>
              Cancelar
            </Button>
            <Button type="button" disabled={busy} onClick={() => void submit()}>
              Dispensar alerta
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      {confirm}
    </>
  );
}
