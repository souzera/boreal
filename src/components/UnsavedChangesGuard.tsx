import { useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useUnsavedChanges } from "@/hooks/useUnsavedChanges";

interface Props {
  isDirty: boolean;
  /** Salva as alterações; devolve `true` se salvou (e a saída pode seguir). */
  onSave?: () => Promise<boolean>;
}

/** Pergunta Salvar / Descartar / Continuar editando ao sair de uma tela com alterações. */
export function UnsavedChangesGuard({ isDirty, onSave }: Props) {
  const blocker = useUnsavedChanges(isDirty);
  const [saving, setSaving] = useState(false);

  const save = async () => {
    if (!onSave) return;
    setSaving(true);
    try {
      if (await onSave()) blocker.proceed?.();
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={blocker.state === "blocked"} onOpenChange={(next) => !next && blocker.reset?.()}>
      <DialogContent showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>Alterações não salvas</DialogTitle>
          <DialogDescription>
            Você tem alterações que ainda não foram salvas. O que deseja fazer?
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={() => blocker.reset?.()}>
            Continuar editando
          </Button>
          <Button variant="destructive" onClick={() => blocker.proceed?.()}>
            Descartar
          </Button>
          {onSave && (
            <Button disabled={saving} onClick={save}>
              Salvar
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
