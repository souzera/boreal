import { useEffect, useState } from "react";

import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Textarea } from "@/components/ui/textarea";
import { useGuardedClose } from "@/hooks/useGuardedClose";
import { releaseBlock, type ApiError, type RoomBlock } from "@/lib/api";
import { useSessionStore } from "@/stores/session";

interface Props {
  open: boolean;
  block: RoomBlock | null;
  onClose: () => void;
  onDone: (message: string) => void;
}

/** Libera o quarto: ele volta a aceitar reservas e hóspedes. Exige justificativa. */
export default function ReleaseDialog({ open, block, onClose, onDone }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const [justification, setJustification] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    setJustification("");
    setError(null);
    setMessage(null);
  }, [open, block]);

  const { requestClose, confirm } = useGuardedClose(justification !== "", onClose);
  if (!block) return null;

  const submit = async () => {
    setMessage(null);
    if (!justification.trim()) {
      setError("Informe a justificativa da liberação.");
      return;
    }
    setError(null);
    setBusy(true);
    try {
      await releaseBlock(token, block.id, justification.trim(), block.version);
      onDone(`Quarto ${block.room_code} liberado.`);
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
            <DialogTitle>{`Liberar quarto ${block.room_code}`}</DialogTitle>
            <DialogDescription>O quarto volta a aceitar reservas e hóspedes. A liberação fica registrada com a justificativa.</DialogDescription>
          </DialogHeader>
          <div className="grid gap-3">
            <p className="rounded-lg border p-2 text-sm">{block.reason}</p>
            <FormField id="rl-justification" label="Justificativa da liberação" required error={error ?? undefined}>
              <Textarea id="rl-justification" value={justification} onChange={(e) => setJustification(e.target.value)} />
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
            <Button type="button" disabled={busy} onClick={submit}>
              Liberar quarto
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      {confirm}
    </>
  );
}
