import type { ReactNode } from "react";

interface Props {
  title: string;
  /** Resume o que o gráfico mostra; vira o texto alternativo da imagem. */
  description: string;
  /** Sem movimento: mostra o aviso no lugar do gráfico e da tabela. */
  empty?: boolean;
  children: ReactNode;
  /** Tabela equivalente ao gráfico, com os mesmos números. */
  table: ReactNode;
}

/** Cartão de gráfico: a cor nunca é a única informação, os números ficam numa tabela ao lado. */
export function ChartCard({ title, description, empty, children, table }: Props) {
  return (
    <section aria-label={title} className="rounded-xl border bg-card p-4 shadow-card">
      <h2 className="text-lg font-semibold">{title}</h2>
      <p className="mb-3 text-sm text-muted-foreground">{description}</p>
      {empty ? (
        <p className="text-sm text-muted-foreground">Sem movimento no período.</p>
      ) : (
        <>
          <div role="img" aria-label={`${title}. ${description}`} className="h-56 w-full">
            {children}
          </div>
          <details className="mt-3 text-sm">
            <summary className="cursor-pointer font-medium">Ver os números</summary>
            <div className="mt-2 overflow-x-auto">{table}</div>
          </details>
        </>
      )}
    </section>
  );
}
