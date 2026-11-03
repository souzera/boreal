import { useQuery } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { Link, useSearchParams } from "react-router-dom";

import Icon from "@/components/Icon";
import { PageHeader } from "@/components/PageHeader";
import { globalSearch, type ApiError, type SearchGroup } from "@/lib/api";
import { formatDate, formatInstant, formatMoney, formatPhone } from "@/lib/format";
import { statusIcon, statusLabel } from "@/lib/reservation";
import { stayStatusIcon, stayStatusLabel } from "@/lib/stay";
import { useSessionStore } from "@/stores/session";

const MIN_CHARS = 2;
const rowClass = "flex flex-wrap items-center justify-between gap-2 rounded-xl border bg-card p-3 text-sm shadow-card hover:bg-muted";

function Section<T>({ title, group, children }: { title: string; group: SearchGroup<T>; children: (item: T) => ReactNode }) {
  if (group.items.length === 0) return null;
  return (
    <section aria-label={title} className="grid gap-2">
      <h2 className="text-sm font-semibold">{`${title} (${group.total})`}</h2>
      <ul className="grid gap-2">{group.items.map((item, i) => <li key={i}>{children(item)}</li>)}</ul>
      {group.total > group.items.length && (
        <p className="text-sm text-muted-foreground">{`Mostrando ${group.items.length} de ${group.total}. Refine a busca para ver os demais.`}</p>
      )}
    </section>
  );
}

/** Resultados da busca geral: hóspedes, reservas, hospedagens e movimentos, cada um com link para o registro. */
export default function Search() {
  const token = useSessionStore((s) => s.token) ?? "";
  const [params] = useSearchParams();
  const text = (params.get("q") ?? "").trim();
  const long = [...text].length >= MIN_CHARS;
  const { data, isPending, error } = useQuery({
    queryKey: ["search", text],
    queryFn: () => globalSearch(token, text),
    enabled: long,
    retry: false,
  });
  const empty = data && data.guests.total + data.reservations.total + data.stays.total + data.movements.total === 0;

  return (
    <section className="grid max-w-3xl gap-4">
      <PageHeader title="Busca" description={long ? `Resultados para “${text}”` : undefined} />
      {text === "" && (
        <p className="text-sm text-muted-foreground">Digite um nome, telefone, CPF, código ou número do quarto no campo de busca.</p>
      )}
      {text !== "" && !long && <p className="text-sm text-muted-foreground">Digite pelo menos 2 letras ou números no campo de busca.</p>}
      {long && error && (
        <p role="alert" className="text-sm text-destructive">
          {(error as ApiError).message}
        </p>
      )}
      {long && !isPending && empty && (
        <div className="grid gap-1 text-sm">
          <p>{`Nenhum resultado para “${text}”.`}</p>
          <p className="text-muted-foreground">
            Tente outro nome, o telefone, o CPF ou um código como RES-000012 ou PAG-000003, ou o número do quarto.
          </p>
        </div>
      )}
      {data && !empty && (
        <>
          <Section title="Hóspedes" group={data.guests}>
            {(g) => (
              <Link to={`/hospedes/${g.id}`} className={rowClass}>
                <strong>{g.name}</strong>
                <span className="flex flex-wrap gap-2 text-muted-foreground">
                  <span>{g.code}</span>
                  <span>{formatPhone(g.phone)}</span>
                  {g.cpf_masked && <span>{g.cpf_masked}</span>}
                  {!g.active && <span>inativo</span>}
                </span>
              </Link>
            )}
          </Section>
          <Section title="Reservas" group={data.reservations}>
            {(r) => (
              <Link to={`/reservas/${r.id}`} className={rowClass}>
                <strong>{`${r.code} — ${r.guest_name}`}</strong>
                <span className="flex flex-wrap gap-2 text-muted-foreground">
                  <span>{`Quarto ${r.room_code}`}</span>
                  <span>{`${formatDate(r.check_in_date)} a ${formatDate(r.check_out_date)}`}</span>
                  <span>
                    <Icon name={statusIcon[r.status]} size="xs" className="mr-1 align-middle" />
                    {statusLabel[r.status]}
                  </span>
                </span>
              </Link>
            )}
          </Section>
          <Section title="Hospedagens" group={data.stays}>
            {(s) => (
              <Link to={`/hospedagens/${s.id}`} className={rowClass}>
                <strong>{`${s.code} — ${s.guest_name}`}</strong>
                <span className="flex flex-wrap gap-2 text-muted-foreground">
                  <span>{`Quarto ${s.room_code}`}</span>
                  <span>
                    <Icon name={stayStatusIcon[s.status]} size="xs" className="mr-1 align-middle" />
                    {stayStatusLabel[s.status]}
                  </span>
                </span>
              </Link>
            )}
          </Section>
          <Section title="Movimentos" group={data.movements}>
            {(m) => (
              <Link to={m.link} className={rowClass}>
                <strong>{m.label}</strong>
                <span className="flex flex-wrap gap-2 text-muted-foreground">
                  <span>{formatMoney(m.amount_cents)}</span>
                  <span>{formatInstant(m.at)}</span>
                </span>
              </Link>
            )}
          </Section>
        </>
      )}
    </section>
  );
}
