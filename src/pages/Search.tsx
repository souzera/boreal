import { useQuery } from "@tanstack/react-query";
import { Link, useSearchParams } from "react-router-dom";

import { PageHeader } from "@/components/PageHeader";
import { listGuests } from "@/lib/api";
import { formatPhone } from "@/lib/format";
import { useSessionStore } from "@/stores/session";

/** Resultados da busca geral. Hoje só hóspedes; reservas e movimentos entram com seus módulos. */
export default function Search() {
  const token = useSessionStore((s) => s.token) ?? "";
  const [params] = useSearchParams();
  const text = (params.get("q") ?? "").trim();
  const { data, isPending } = useQuery({
    queryKey: ["search", text],
    queryFn: () => listGuests(token, { text, page: 1, page_size: 10 }),
    enabled: text !== "",
  });

  return (
    <section className="max-w-2xl">
      <PageHeader title="Busca" description={text ? `Resultados para “${text}”` : undefined} />
      {text === "" && (
        <p className="text-sm text-muted-foreground">
          Digite um nome, telefone, CPF ou código no campo de busca para encontrar um hóspede.
        </p>
      )}
      {text !== "" && !isPending && data?.items.length === 0 && (
        <p className="text-sm">{`Nenhum resultado para “${text}”.`}</p>
      )}
      {data && data.items.length > 0 && (
        <div className="grid gap-2">
          <h2 className="text-sm font-semibold">Hóspedes</h2>
          <ul className="grid gap-2">
            {data.items.map((g) => (
              <li key={g.id}>
                <Link to={`/hospedes/${g.id}`} className="flex flex-wrap items-center justify-between gap-2 rounded-lg border p-3 text-sm hover:bg-muted">
                  <strong>{g.full_name}</strong>
                  <span className="flex gap-2 text-muted-foreground">
                    <span>{g.code}</span>
                    <span>{formatPhone(g.phone)}</span>
                    {!g.active && <span>inativo</span>}
                  </span>
                </Link>
              </li>
            ))}
          </ul>
          {data.total > data.items.length && (
            <p className="text-sm text-muted-foreground">
              Mostrando {data.items.length} de {data.total}. Refine a busca ou abra a lista de hóspedes.
            </p>
          )}
        </div>
      )}
    </section>
  );
}
