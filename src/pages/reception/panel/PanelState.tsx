import type { ReactNode } from "react";

/** Mensagem padrão do painel quando o registro não carrega (id inexistente ou falha). */
export function PanelError() {
  return (
    <p role="alert" className="text-sm text-destructive">
      Não foi possível carregar este registro.
    </p>
  );
}

export function PanelLine({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex flex-wrap justify-between gap-2">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="font-medium">{children}</dd>
    </div>
  );
}
