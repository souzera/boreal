import type { ReactNode } from "react";

interface Props {
  title: string;
  description?: string;
  /** Ação principal da tela (um botão com texto específico). */
  action?: ReactNode;
}

export function PageHeader({ title, description, action }: Props) {
  return (
    <header className="mb-6 flex flex-wrap items-start justify-between gap-3">
      <div>
        <h1 className="text-2xl font-semibold">{title}</h1>
        {description && <p className="text-sm text-muted-foreground">{description}</p>}
      </div>
      {action}
    </header>
  );
}
