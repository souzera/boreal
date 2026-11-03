import type { ReactNode } from "react";

interface Props {
  title: string;
  description?: ReactNode;
  /** Ação principal da tela (um botão com texto específico). */
  action?: ReactNode;
}

export function PageHeader({ title, description, action }: Props) {
  return (
    <header className="mb-6 flex flex-wrap items-start justify-between gap-3">
      <div>
        <h1 className="text-[26px] font-[650] leading-tight">{title}</h1>
        {description && <div className="text-sm text-muted-foreground">{description}</div>}
      </div>
      {action}
    </header>
  );
}
