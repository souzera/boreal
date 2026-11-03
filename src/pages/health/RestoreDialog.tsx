import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import Icon from "@/components/Icon";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { previewRestore, restoreBackup, type ApiError, type BackupCounts } from "@/lib/api";
import { CONFIRMATION_WORD } from "@/lib/backup";
import { useSessionStore } from "@/stores/session";

interface Props {
  /** Caminho completo do arquivo de backup escolhido. */
  path: string | null;
  onClose: () => void;
  /** Chamado depois de restaurar: todas as sessões foram encerradas. */
  onRestored: () => void;
}

const countsText = (c: BackupCounts) => `${c.reservations} reservas, ${c.stays} hospedagens, ${c.payments} recebimentos, ${c.guests} hóspedes`;

/** Restauração com prévia em texto, confirmação digitada e senha (spec §15.2). */
export function RestoreDialog({ path, onClose, onRestored }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const [confirmation, setConfirmation] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const { data: preview, isError } = useQuery({
    queryKey: ["restore-preview", path],
    queryFn: () => previewRestore(token, path as string),
    enabled: !!path && !!token,
    retry: false,
  });

  const ready = !!preview?.restorable && confirmation.trim().toUpperCase() === CONFIRMATION_WORD && password.length > 0 && !busy;

  const submit = async () => {
    if (!path) return;
    setBusy(true);
    setError(null);
    try {
      await restoreBackup(token, path, confirmation, password);
      setPassword("");
      onRestored();
    } catch (e) {
      setPassword("");
      setError((e as ApiError).message);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog open={!!path} onOpenChange={(open) => !open && !busy && onClose()}>
      <DialogContent showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>Restaurar um backup</DialogTitle>
          <DialogDescription>O sistema volta ao estado do backup. Uma cópia do estado atual é guardada antes.</DialogDescription>
        </DialogHeader>

        {isError && (
          <p role="alert" className="text-sm text-destructive">
            Não foi possível ler este backup.
          </p>
        )}
        {preview && (
          <div className="grid gap-2 text-sm">
            <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1">
              <dt className="text-muted-foreground">Backup de</dt>
              <dd>{preview.backup_at}</dd>
              <dt className="text-muted-foreground">Contém</dt>
              <dd>{countsText(preview.counts)}</dd>
              <dt className="text-muted-foreground">Agora o sistema tem</dt>
              <dd>{countsText(preview.current_counts)}</dd>
            </dl>
            {!preview.restorable && (
              <p role="alert" className="flex items-start gap-1 text-destructive">
                <Icon name="error" size="sm" />
                <span>{`Este backup não pode ser restaurado: ${preview.problem ?? "arquivo inválido."}`}</span>
              </p>
            )}
            {preview.restorable && preview.loses_recent_data && (
              <p className="flex items-start gap-1 text-warning-foreground">
                <Icon name="warning" size="sm" />
                <span>Há atividade no sistema depois deste backup. Tudo o que foi feito depois dele será perdido (a cópia de segurança guarda o estado atual).</span>
              </p>
            )}
          </div>
        )}

        {preview?.restorable && (
          <div className="grid gap-3">
            <label className="grid gap-1 text-sm">
              {`Digite ${CONFIRMATION_WORD} para confirmar`}
              <Input value={confirmation} autoComplete="off" onChange={(e) => setConfirmation(e.target.value)} />
            </label>
            <label className="grid gap-1 text-sm">
              Sua senha
              <Input type="password" value={password} autoComplete="current-password" onChange={(e) => setPassword(e.target.value)} />
            </label>
            <p className="text-sm text-muted-foreground">Depois de restaurar, todas as pessoas precisam entrar de novo.</p>
          </div>
        )}
        {error && (
          <p role="alert" className="text-sm text-destructive">
            {error}
          </p>
        )}

        <DialogFooter>
          <Button variant="outline" disabled={busy} onClick={onClose}>
            Cancelar
          </Button>
          <Button variant="destructive" disabled={!ready} onClick={() => void submit()}>
            Restaurar este backup
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
