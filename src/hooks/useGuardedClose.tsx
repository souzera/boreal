import { useState } from "react";

import { ConfirmDialog } from "@/components/ConfirmDialog";

/**
 * Fechar um formulário em diálogo (Esc, clique fora ou Cancelar) com dados digitados pergunta
 * antes de descartar (spec §18). Renderize `confirm` junto do diálogo.
 */
export function useGuardedClose(isDirty: boolean, onClose: () => void) {
  const [asking, setAsking] = useState(false);

  const requestClose = () => {
    if (isDirty) setAsking(true);
    else onClose();
  };

  const confirm = (
    <ConfirmDialog
      open={asking}
      title="Descartar as alterações?"
      description="O que foi digitado será perdido."
      confirmLabel="Descartar alterações"
      cancelLabel="Continuar editando"
      destructive
      onCancel={() => setAsking(false)}
      onConfirm={() => {
        setAsking(false);
        onClose();
      }}
    />
  );

  return { requestClose, confirm };
}
