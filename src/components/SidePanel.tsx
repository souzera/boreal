import type { ReactNode } from "react";

import { Button } from "@/components/ui/button";

interface Props {
  title: string;
  onClose: () => void;
  children: ReactNode;
}

/**
 * Painel ao lado da agenda. Não é modal: não prende o foco nem esconde o resto da tela,
 * para que abrir um registro não faça a pessoa perder o mapa em que estava.
 */
export function SidePanel({ title, onClose, children }: Props) {
  return (
    <aside
      aria-label={title}
      className="rounded-lg border bg-card p-4"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.stopPropagation();
          onClose();
        }
      }}
    >
      <div className="mb-3 flex items-start justify-between gap-2">
        <h2 className="text-base font-semibold">{title}</h2>
        <Button type="button" variant="outline" size="sm" onClick={onClose}>
          Fechar painel
        </Button>
      </div>
      {children}
    </aside>
  );
}
