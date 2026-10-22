import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import { DataTable, type ColumnDef } from "./DataTable";

interface Row {
  id: number;
  name: string;
  capacity: number;
}

const columns: ColumnDef<Row>[] = [
  { key: "name", header: "Nome", sortable: true, render: (r) => r.name },
  { key: "capacity", header: "Capacidade", sortable: true, render: (r) => r.capacity },
  { key: "note", header: "Obs.", render: () => "—" },
];

const rows: Row[] = [
  { id: 1, name: "Quarto 101", capacity: 2 },
  { id: 2, name: "Quarto 102", capacity: 4 },
];

function setup(props: Partial<React.ComponentProps<typeof DataTable<Row>>> = {}) {
  const handlers = {
    onPageChange: vi.fn(),
    onSortChange: vi.fn(),
    onRowClick: vi.fn(),
  };
  render(
    <DataTable
      columns={columns}
      rows={rows}
      getRowKey={(r) => r.id}
      total={45}
      page={2}
      pageSize={20}
      sort={{ key: "name", direction: "asc" }}
      emptyMessage="Nenhum quarto encontrado."
      {...handlers}
      {...props}
    />,
  );
  return handlers;
}

describe("DataTable", () => {
  it("mostra cabeçalhos, linhas e o resumo da página", () => {
    setup();

    expect(screen.getByRole("columnheader", { name: /Nome/ })).toBeInTheDocument();
    expect(screen.getByText("Quarto 101")).toBeInTheDocument();
    expect(screen.getByText("Quarto 102")).toBeInTheDocument();
    expect(screen.getByText("Mostrando 21–40 de 45")).toBeInTheDocument();
  });

  it("indica a coluna ordenada com aria-sort", () => {
    setup();

    expect(screen.getByRole("columnheader", { name: /Nome/ })).toHaveAttribute("aria-sort", "ascending");
    expect(screen.getByRole("columnheader", { name: /Capacidade/ })).toHaveAttribute("aria-sort", "none");
  });

  it("clicar no cabeçalho ordenável pede a ordenação; inverte quando já ordenada", async () => {
    const { onSortChange } = setup();

    await userEvent.click(within(screen.getByRole("columnheader", { name: /Capacidade/ })).getByRole("button"));
    expect(onSortChange).toHaveBeenLastCalledWith({ key: "capacity", direction: "asc" });

    await userEvent.click(within(screen.getByRole("columnheader", { name: /Nome/ })).getByRole("button"));
    expect(onSortChange).toHaveBeenLastCalledWith({ key: "name", direction: "desc" });
  });

  it("coluna sem sortable não tem botão", () => {
    setup();

    expect(within(screen.getByRole("columnheader", { name: /Obs\./ })).queryByRole("button")).toBeNull();
  });

  it("paginação chama onPageChange e desabilita os limites", async () => {
    const { onPageChange } = setup();

    await userEvent.click(screen.getByRole("button", { name: "Próxima" }));
    expect(onPageChange).toHaveBeenLastCalledWith(3);
    await userEvent.click(screen.getByRole("button", { name: "Anterior" }));
    expect(onPageChange).toHaveBeenLastCalledWith(1);
  });

  it("na primeira e na última página o botão correspondente fica desabilitado", () => {
    setup({ page: 1 });
    expect(screen.getByRole("button", { name: "Anterior" })).toBeDisabled();
  });

  it("sem linhas mostra a mensagem vazia", () => {
    setup({ rows: [], total: 0, page: 1 });

    expect(screen.getByText("Nenhum quarto encontrado.")).toBeInTheDocument();
    expect(screen.queryByText(/Mostrando/)).toBeNull();
  });

  it("clicar na linha chama onRowClick com a linha", async () => {
    const { onRowClick } = setup();

    await userEvent.click(screen.getByText("Quarto 102"));

    expect(onRowClick).toHaveBeenCalledWith(rows[1]);
  });
});
