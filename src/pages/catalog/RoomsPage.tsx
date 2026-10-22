import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { Link } from "react-router-dom";

import { ConfirmDialog } from "@/components/ConfirmDialog";
import { DataTable, type ColumnDef, type SortState } from "@/components/data-table/DataTable";
import { FormField } from "@/components/FormField";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  createRoom,
  listCategories,
  listRooms,
  setRoomActive,
  updateRoom,
  type ApiError,
  type Room,
  type RoomInput,
} from "@/lib/api";
import { useSessionStore } from "@/stores/session";
import RoomForm from "./RoomForm";

const PAGE_SIZE = 20;

export default function RoomsPage() {
  const token = useSessionStore((s) => s.token) ?? "";
  const canManage = useSessionStore((s) => s.user?.role === "admin");
  const queryClient = useQueryClient();
  const [text, setText] = useState("");
  const [categoryId, setCategoryId] = useState<number | undefined>();
  const [page, setPage] = useState(1);
  const [sort, setSort] = useState<SortState>({ key: "code", direction: "asc" });
  const [form, setForm] = useState<{ open: boolean; room: Room | null }>({ open: false, room: null });
  const [toDeactivate, setToDeactivate] = useState<Room | null>(null);
  const [notice, setNotice] = useState<{ kind: "ok" | "error"; text: string } | null>(null);

  const query = {
    text: text || undefined,
    category_id: categoryId,
    page,
    page_size: PAGE_SIZE,
    sort: sort.key,
    direction: sort.direction,
  };
  const { data } = useQuery({ queryKey: ["rooms", query], queryFn: () => listRooms(token, query) });
  const { data: categories = [] } = useQuery({
    queryKey: ["categories", "all"],
    queryFn: () => listCategories(token, false),
  });
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["rooms"] });

  const run = async (action: () => Promise<unknown>, success: string) => {
    setNotice(null);
    try {
      await action();
      await refresh();
      setNotice({ kind: "ok", text: success });
    } catch (e) {
      setNotice({ kind: "error", text: (e as ApiError).message });
    }
  };

  const save = async (input: RoomInput) => {
    const editing = form.room;
    if (editing) await updateRoom(token, editing.id, input, editing.version);
    else await createRoom(token, input);
    await refresh();
    setForm({ open: false, room: null });
    setNotice({ kind: "ok", text: editing ? `Quarto ${input.code} atualizado.` : `Quarto ${input.code} criado.` });
  };

  const columns: ColumnDef<Room>[] = [
    { key: "code", header: "Quarto", sortable: true, render: (r) => <strong>{r.code}</strong> },
    { key: "category_name", header: "Categoria", sortable: true, render: (r) => r.category_name },
    {
      key: "max_capacity",
      header: "Capacidade",
      sortable: true,
      render: (r) => `${r.max_capacity} ${r.max_capacity === 1 ? "pessoa" : "pessoas"}`,
    },
    { key: "description", header: "Descrição", render: (r) => r.description || "—" },
    { key: "active", header: "Situação", sortable: true, render: (r) => (r.active ? "Ativo" : "Inativo") },
  ];
  if (canManage) {
    columns.push({
      key: "actions",
      header: "Ações",
      render: (r) => (
        <div className="flex gap-1">
          <Button size="sm" variant="outline" onClick={() => setForm({ open: true, room: r })}>
            Editar
          </Button>
          {r.active ? (
            <Button size="sm" variant="outline" onClick={() => setToDeactivate(r)}>
              Desativar
            </Button>
          ) : (
            <Button
              size="sm"
              variant="outline"
              onClick={() => run(() => setRoomActive(token, r.id, true), `Quarto ${r.code} reativado.`)}
            >
              Reativar
            </Button>
          )}
        </div>
      ),
    });
  }

  return (
    <section>
      <PageHeader
        title="Quartos"
        description="Quartos da pousada e suas categorias."
        action={
          <div className="flex gap-2">
            <Button variant="outline" render={<Link to="/quartos/categorias" />}>
              Categorias
            </Button>
            {canManage && <Button onClick={() => setForm({ open: true, room: null })}>Novo quarto</Button>}
          </div>
        }
      />
      <div className="mb-4 flex flex-wrap gap-4">
        <div className="w-56">
          <FormField id="room-search" label="Buscar quarto">
            <Input
              id="room-search"
              value={text}
              onChange={(e) => {
                setText(e.target.value);
                setPage(1);
              }}
            />
          </FormField>
        </div>
        <div className="w-56">
          <FormField id="room-filter-category" label="Categoria do filtro">
            <select
              id="room-filter-category"
              className="h-8 rounded-lg border border-input bg-transparent px-2 text-sm"
              value={categoryId ?? ""}
              onChange={(e) => {
                setCategoryId(e.target.value ? Number(e.target.value) : undefined);
                setPage(1);
              }}
            >
              <option value="">Todas</option>
              {categories.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </select>
          </FormField>
        </div>
      </div>
      {notice && (
        <p role={notice.kind === "ok" ? "status" : "alert"} className={notice.kind === "ok" ? "mb-3 text-sm" : "mb-3 text-sm text-destructive"}>
          {notice.text}
        </p>
      )}
      <DataTable
        columns={columns}
        rows={data?.items ?? []}
        getRowKey={(r) => r.id}
        total={data?.total ?? 0}
        page={page}
        pageSize={PAGE_SIZE}
        sort={sort}
        emptyMessage="Nenhum quarto encontrado."
        onPageChange={setPage}
        onSortChange={(next) => {
          setSort(next);
          setPage(1);
        }}
      />
      <RoomForm
        open={form.open}
        room={form.room}
        categories={categories}
        onClose={() => setForm({ open: false, room: null })}
        onSubmit={save}
      />
      <ConfirmDialog
        open={toDeactivate !== null}
        title="Desativar quarto?"
        description={`O quarto ${toDeactivate?.code} deixa de aparecer para novas reservas. O histórico é mantido.`}
        confirmLabel="Desativar quarto"
        destructive
        onCancel={() => setToDeactivate(null)}
        onConfirm={async () => {
          const room = toDeactivate;
          setToDeactivate(null);
          if (room) await run(() => setRoomActive(token, room.id, false), `Quarto ${room.code} desativado.`);
        }}
      />
    </section>
  );
}
