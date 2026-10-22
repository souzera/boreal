import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";

import { DataTable, type ColumnDef, type SortState } from "@/components/data-table/DataTable";
import { FormField } from "@/components/FormField";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { listGuests, type Guest } from "@/lib/api";
import { formatCpf, formatPhone } from "@/lib/format";
import { useSessionStore } from "@/stores/session";

const PAGE_SIZE = 20;

export default function GuestsPage() {
  const token = useSessionStore((s) => s.token) ?? "";
  const navigate = useNavigate();
  const [text, setText] = useState("");
  const [page, setPage] = useState(1);
  const [sort, setSort] = useState<SortState>({ key: "name_key", direction: "asc" });

  const query = { text: text || undefined, page, page_size: PAGE_SIZE, sort: sort.key, direction: sort.direction };
  const { data } = useQuery({ queryKey: ["guests", query], queryFn: () => listGuests(token, query) });

  const columns: ColumnDef<Guest>[] = [
    { key: "name_key", header: "Nome", sortable: true, render: (g) => <strong>{g.full_name}</strong> },
    { key: "code", header: "Código", sortable: true, render: (g) => g.code },
    { key: "phone", header: "Telefone", sortable: true, render: (g) => formatPhone(g.phone) },
    { key: "cpf", header: "CPF", render: (g) => (g.cpf ? formatCpf(g.cpf) : "—") },
    { key: "active", header: "Situação", sortable: true, render: (g) => (g.active ? "Ativo" : "Inativo") },
  ];

  return (
    <section>
      <PageHeader
        title="Hóspedes"
        description="Cadastro e histórico de hóspedes."
        action={
          <Button render={<Link to="/hospedes/novo" />}>Novo hóspede</Button>
        }
      />
      <div className="mb-4 max-w-sm">
        <FormField id="guest-search" label="Buscar hóspede">
          <Input
            id="guest-search"
            placeholder="Nome, telefone, CPF ou código"
            value={text}
            onChange={(e) => {
              setText(e.target.value);
              setPage(1);
            }}
          />
        </FormField>
      </div>
      <DataTable
        columns={columns}
        rows={data?.items ?? []}
        getRowKey={(g) => g.id}
        total={data?.total ?? 0}
        page={page}
        pageSize={PAGE_SIZE}
        sort={sort}
        emptyMessage="Nenhum hóspede encontrado."
        onPageChange={setPage}
        onSortChange={(next) => {
          setSort(next);
          setPage(1);
        }}
        onRowClick={(g) => navigate(`/hospedes/${g.id}`)}
      />
    </section>
  );
}
