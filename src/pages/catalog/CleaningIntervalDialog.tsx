import { useEffect, useState } from "react";

import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { setRoomCleaningInterval, type ApiError, type Room } from "@/lib/api";
import { useSessionStore } from "@/stores/session";

interface Props {
  room: Room | null;
  onClose: () => void;
  onDone: (message: string) => void;
}

const LIMIT_MESSAGE = "Informe de 0 a 1440 minutos, ou deixe em branco para usar o padrão.";

/** Intervalo mínimo de limpeza próprio do quarto; em branco volta ao padrão da pousada. */
export default function CleaningIntervalDialog({ room, onClose, onDone }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    setValue(room?.cleaning_interval_minutes != null ? String(room.cleaning_interval_minutes) : "");
    setError(null);
    setMessage(null);
  }, [room]);

  if (!room) return null;

  const submit = async () => {
    setMessage(null);
    const text = value.trim();
    let minutes: number | null = null;
    if (text !== "") {
      if (!/^\d+$/.test(text) || Number(text) > 1440) {
        setError(LIMIT_MESSAGE);
        return;
      }
      minutes = Number(text);
    }
    setError(null);
    setBusy(true);
    try {
      await setRoomCleaningInterval(token, room.id, minutes, room.version);
      onDone(`Intervalo de limpeza do quarto ${room.code} atualizado.`);
    } catch (e) {
      setMessage((e as ApiError).message);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog open onOpenChange={(next) => !next && onClose()}>
      <DialogContent showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>{`Intervalo de limpeza do quarto ${room.code}`}</DialogTitle>
          <DialogDescription>
            Tempo mínimo entre a limpeza concluída e a próxima hospedagem. Em branco, vale o padrão da pousada.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-3">
          <FormField id="ci-minutes" label="Intervalo mínimo (minutos)" error={error ?? undefined}>
            <Input id="ci-minutes" inputMode="numeric" value={value} onChange={(e) => setValue(e.target.value)} />
          </FormField>
          {message && (
            <p role="alert" className="text-sm text-destructive">
              {message}
            </p>
          )}
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={onClose}>
            Cancelar
          </Button>
          <Button type="button" disabled={busy} onClick={submit}>
            Salvar intervalo
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
