import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { FormField, RequiredLegend } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { useGuardedClose } from "@/hooks/useGuardedClose";
import { createBlock, listRooms, type ApiError } from "@/lib/api";
import { useSessionStore } from "@/stores/session";

interface Props {
  open: boolean;
  onClose: () => void;
  onDone: (message: string) => void;
}

type Errors = Partial<Record<"room" | "start" | "end" | "reason", string>>;

/** Bloqueio manual: por período (fim opcional) ou até a liberação manual. Reservas afetadas exigem ordem de manutenção. */
export default function BlockDialog({ open, onClose, onDone }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const [roomId, setRoomId] = useState("");
  const [start, setStart] = useState("");
  const [end, setEnd] = useState("");
  const [reason, setReason] = useState("");
  const [errors, setErrors] = useState<Errors>({});
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const { data: rooms } = useQuery({
    queryKey: ["rooms", "active-for-block"],
    queryFn: () => listRooms(token, { active: true, page: 1, page_size: 100 }),
    enabled: open,
  });

  useEffect(() => {
    if (!open) return;
    setRoomId("");
    setStart("");
    setEnd("");
    setReason("");
    setErrors({});
    setMessage(null);
  }, [open]);

  const { requestClose, confirm } = useGuardedClose(roomId !== "" || start !== "" || end !== "" || reason !== "", onClose);

  const submit = async () => {
    setMessage(null);
    const next: Errors = {};
    if (!roomId) next.room = "Escolha o quarto.";
    if (!start) next.start = "Informe a data de início.";
    if (start && end && end <= start) next.end = "O fim deve ser depois do início.";
    if (!reason.trim()) next.reason = "Informe o motivo do bloqueio.";
    setErrors(next);
    if (Object.keys(next).length > 0) return;

    setBusy(true);
    try {
      await createBlock(token, { room_id: Number(roomId), start_date: start, end_date: end || null, reason: reason.trim() });
      const code = rooms?.items.find((r) => r.id === Number(roomId))?.code ?? "";
      onDone(`Quarto ${code} bloqueado.`);
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
            <DialogTitle>Criar bloqueio</DialogTitle>
            <DialogDescription>
              O quarto deixa de aceitar reservas e hóspedes no período. Sem data de fim, vale até alguém liberar o quarto.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-3">
            <FormField id="bk-room" label="Quarto" required error={errors.room}>
              <select
                id="bk-room"
                className="h-8 rounded-lg border border-input bg-transparent px-2 text-sm"
                value={roomId}
                onChange={(e) => setRoomId(e.target.value)}
              >
                <option value="">Escolha o quarto</option>
                {rooms?.items.map((r) => (
                  <option key={r.id} value={r.id}>
                    {r.code}
                  </option>
                ))}
              </select>
            </FormField>
            <FormField id="bk-start" label="Início" required error={errors.start}>
              <Input id="bk-start" type="date" value={start} onChange={(e) => setStart(e.target.value)} />
            </FormField>
            <FormField id="bk-end" label="Fim (opcional, a noite do fim não é bloqueada)" error={errors.end}>
              <Input id="bk-end" type="date" value={end} onChange={(e) => setEnd(e.target.value)} />
            </FormField>
            <FormField id="bk-reason" label="Motivo" required error={errors.reason}>
              <Textarea id="bk-reason" value={reason} onChange={(e) => setReason(e.target.value)} />
            </FormField>
            <RequiredLegend />
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
              Criar bloqueio
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      {confirm}
    </>
  );
}
