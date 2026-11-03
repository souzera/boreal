import { useEffect, useRef, type ReactNode } from "react";

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
  const ref = useRef<HTMLElement>(null);
  // O foco vai para o painel ao abrir (e ao trocar de registro), para o Esc funcionar sem clicar nele.
  useEffect(() => {
    ref.current?.focus();
  }, [title]);

  return (
    <aside
      ref={ref}
      tabIndex={-1}
      aria-label={title}
      className="rounded-lg border bg-card p-4 outline-none focus-visible:ring-2 focus-visible:ring-ring"
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
