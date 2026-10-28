import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { FormField, RequiredLegend } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { useGuardedClose } from "@/hooks/useGuardedClose";
import { listRooms, openMaintenance, type ApiError, type CleaningPriority, type OpenedOrder } from "@/lib/api";
import { orderPriorityLabel } from "@/lib/blocks";
import { useSessionStore } from "@/stores/session";

interface Props {
  open: boolean;
  onClose: () => void;
  onDone: (opened: OpenedOrder) => void;
}

type Errors = Partial<Record<"room" | "description" | "start" | "end", string>>;

const PRIORITIES: CleaningPriority[] = ["urgent", "high", "normal", "low"];

/** Abre uma ordem de manutenção. Reservas afetadas viram um processo de resolução (bloqueio preventivo). */
export default function OrderDialog({ open, onClose, onDone }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const [roomId, setRoomId] = useState("");
  const [description, setDescription] = useState("");
  const [priority, setPriority] = useState<CleaningPriority>("normal");
  const [start, setStart] = useState("");
  const [end, setEnd] = useState("");
  const [responsible, setResponsible] = useState("");
  const [notes, setNotes] = useState("");
  const [errors, setErrors] = useState<Errors>({});
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const { data: rooms } = useQuery({
    queryKey: ["rooms", "active-for-order"],
    queryFn: () => listRooms(token, { active: true, page: 1, page_size: 100 }),
    enabled: open,
  });

  useEffect(() => {
    if (!open) return;
    setRoomId("");
    setDescription("");
    setPriority("normal");
    setStart("");
    setEnd("");
    setResponsible("");
    setNotes("");
    setErrors({});
    setMessage(null);
  }, [open]);

  const dirty = roomId !== "" || description !== "" || start !== "" || end !== "" || responsible !== "" || notes !== "";
  const { requestClose, confirm } = useGuardedClose(dirty, onClose);

  const submit = async () => {
    setMessage(null);
    const next: Errors = {};
    if (!roomId) next.room = "Escolha o quarto.";
    if (!description.trim()) next.description = "Descreva o que precisa ser feito.";
    if (!start) next.start = "Informe o início previsto.";
    if (start && end && end <= start) next.end = "O fim deve ser depois do início.";
    setErrors(next);
    if (Object.keys(next).length > 0) return;

    setBusy(true);
    try {
      const opened = await openMaintenance(token, {
        room_id: Number(roomId),
        description: description.trim(),
        priority,
        planned_start: start,
        planned_end: end || null,
        responsible: responsible.trim(),
        notes: notes.trim(),
      });
      onDone(opened);
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
            <DialogTitle>Abrir ordem de manutenção</DialogTitle>
            <DialogDescription>
              Se houver reservas no período, o quarto fica com bloqueio preventivo (sem novas reservas) até elas serem resolvidas.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-3">
            <FormField id="mo-room" label="Quarto" required error={errors.room}>
              <select
                id="mo-room"
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
            <FormField id="mo-description" label="Descrição" required error={errors.description}>
              <Textarea id="mo-description" value={description} onChange={(e) => setDescription(e.target.value)} />
            </FormField>
            <FormField id="mo-priority" label="Prioridade">
              <select
                id="mo-priority"
                className="h-8 rounded-lg border border-input bg-transparent px-2 text-sm"
                value={priority}
                onChange={(e) => setPriority(e.target.value as CleaningPriority)}
              >
                {PRIORITIES.map((p) => (
                  <option key={p} value={p}>
                    {orderPriorityLabel[p]}
                  </option>
                ))}
              </select>
            </FormField>
            <FormField id="mo-start" label="Início previsto" required error={errors.start}>
              <Input id="mo-start" type="date" value={start} onChange={(e) => setStart(e.target.value)} />
            </FormField>
            <FormField id="mo-end" label="Fim previsto (opcional)" error={errors.end}>
              <Input id="mo-end" type="date" value={end} onChange={(e) => setEnd(e.target.value)} />
            </FormField>
            <FormField id="mo-responsible" label="Responsável">
              <Input id="mo-responsible" value={responsible} onChange={(e) => setResponsible(e.target.value)} />
            </FormField>
            <FormField id="mo-notes" label="Observações">
              <Textarea id="mo-notes" value={notes} onChange={(e) => setNotes(e.target.value)} />
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
              Abrir ordem de manutenção
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      {confirm}
    </>
  );
}
