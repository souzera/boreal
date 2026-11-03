import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { ConfirmDialog } from "@/components/ConfirmDialog";
import { DataTable, type ColumnDef, type SortState } from "@/components/data-table/DataTable";
import { FormField } from "@/components/FormField";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  createUser,
  listUsers,
  resetPassword,
  setExtraPermissions,
  setUserActive,
  setUserRole,
  type ApiError,
  type NewUserInput,
  type UserRow,
} from "@/lib/api";
import { useSessionStore } from "@/stores/session";
import { roleLabel } from "./labels";
import PermissionsDialog from "./PermissionsDialog";
import ResetPasswordDialog from "./ResetPasswordDialog";
import UserForm from "./UserForm";

const PAGE_SIZE = 20;

type Pending =
  | { kind: "toggle-active"; user: UserRow }
  | { kind: "change-role"; user: UserRow }
  | null;

export default function UsersPage() {
  const token = useSessionStore((s) => s.token) ?? "";
  const currentUserId = useSessionStore((s) => s.user?.id);
  const queryClient = useQueryClient();
  const [text, setText] = useState("");
  const [page, setPage] = useState(1);
  const [sort, setSort] = useState<SortState>({ key: "display_name", direction: "asc" });
  const [creating, setCreating] = useState(false);
  const [resetting, setResetting] = useState<UserRow | null>(null);
  const [editingPermissions, setEditingPermissions] = useState<UserRow | null>(null);
  const [pending, setPending] = useState<Pending>(null);
  const [notice, setNotice] = useState<{ kind: "ok" | "error"; text: string } | null>(null);

  const query = { text: text || undefined, page, page_size: PAGE_SIZE, sort: sort.key, direction: sort.direction };
  const { data } = useQuery({
    queryKey: ["users", query],
    queryFn: () => listUsers(token, query),
  });
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["users"] });

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

  const columns: ColumnDef<UserRow>[] = [
    { key: "display_name", header: "Nome", sortable: true, render: (u) => u.display_name },
    { key: "username", header: "Usuário", sortable: true, render: (u) => u.username },
    { key: "role", header: "Perfil", sortable: true, render: (u) => roleLabel[u.role] },
    {
      key: "active",
      header: "Situação",
      sortable: true,
      render: (u) => (u.active ? "Ativo" : "Inativo"),
    },
    {
      key: "actions",
      header: "Ações",
      render: (u) => (
        <div className="flex flex-wrap gap-1">
          {u.role === "recepcao" && (
            <Button size="sm" variant="outline" onClick={() => setEditingPermissions(u)}>
              Permissões
            </Button>
          )}
          <Button size="sm" variant="outline" onClick={() => setResetting(u)}>
            Redefinir senha
          </Button>
          {u.id !== currentUserId && (
            <>
              <Button size="sm" variant="outline" onClick={() => setPending({ kind: "change-role", user: u })}>
                Alterar perfil
              </Button>
              <Button size="sm" variant="outline" onClick={() => setPending({ kind: "toggle-active", user: u })}>
                {u.active ? "Desativar" : "Reativar"}
              </Button>
            </>
          )}
        </div>
      ),
    },
  ];

  return (
    <section>
      <PageHeader
        title="Usuários"
        description="Quem pode entrar no sistema e o que cada pessoa pode fazer."
        action={<Button onClick={() => setCreating(true)}>Novo usuário</Button>}
      />
      <div className="mb-4 max-w-xs">
        <FormField id="user-search" label="Buscar usuário">
          <Input
            id="user-search"
            value={text}
            onChange={(e) => {
              setText(e.target.value);
              setPage(1);
            }}
          />
        </FormField>
      </div>
      {notice && (
        <p role={notice.kind === "ok" ? "status" : "alert"} className={notice.kind === "ok" ? "mb-3 text-sm" : "mb-3 text-sm text-destructive"}>
          {notice.text}
        </p>
      )}
      <DataTable
        columns={columns}
        rows={data?.items ?? []}
        getRowKey={(u) => u.id}
        total={data?.total ?? 0}
        page={page}
        pageSize={PAGE_SIZE}
        sort={sort}
        emptyMessage="Nenhum usuário encontrado."
        onPageChange={setPage}
        onSortChange={(next) => {
          setSort(next);
          setPage(1);
        }}
      />

      <UserForm
        open={creating}
        onClose={() => setCreating(false)}
        onSubmit={async (input: NewUserInput) => {
          await createUser(token, input);
          await refresh();
          setCreating(false);
          setNotice({
            kind: "ok",
            text: `Usuário ${input.username} criado. A senha temporária deve ser trocada no primeiro acesso.`,
          });
        }}
      />
      <ResetPasswordDialog
        user={resetting}
        onClose={() => setResetting(null)}
        onSubmit={async (user, tempPassword) => {
          await resetPassword(token, user.id, tempPassword, user.version);
          await refresh();
          setResetting(null);
          setNotice({ kind: "ok", text: `Senha de ${user.display_name} redefinida.` });
        }}
      />
      <PermissionsDialog
        user={editingPermissions}
        onClose={() => setEditingPermissions(null)}
        onSubmit={async (user, permissions) => {
          await setExtraPermissions(token, user.id, permissions);
          await refresh();
          setEditingPermissions(null);
          setNotice({ kind: "ok", text: `Permissões de ${user.display_name} salvas.` });
        }}
      />
      <ConfirmDialog
        open={pending?.kind === "toggle-active"}
        title={pending?.user.active ? "Desativar usuário?" : "Reativar usuário?"}
        description={
          pending?.user.active
            ? `${pending.user.display_name} não conseguirá mais entrar e as sessões abertas serão encerradas. O histórico é mantido.`
            : `${pending?.user.display_name} poderá entrar novamente.`
        }
        confirmLabel={pending?.user.active ? "Desativar usuário" : "Reativar usuário"}
        destructive={pending?.user.active}
        onCancel={() => setPending(null)}
        onConfirm={async () => {
          const user = pending?.user;
          setPending(null);
          if (user) await run(() => setUserActive(token, user.id, !user.active), `${user.display_name} ${user.active ? "desativado" : "reativado"}.`);
        }}
      />
      <ConfirmDialog
        open={pending?.kind === "change-role"}
        title="Alterar perfil?"
        description={
          pending?.user.role === "recepcao"
            ? `${pending.user.display_name} passará a ter todas as permissões de administrador.`
            : `${pending?.user.display_name} passará a ter o perfil de recepção e perderá as permissões de administrador.`
        }
        confirmLabel={pending?.user.role === "recepcao" ? "Tornar administrador" : "Tornar recepção"}
        onCancel={() => setPending(null)}
        onConfirm={async () => {
          const user = pending?.user;
          setPending(null);
          if (user) {
            const role = user.role === "recepcao" ? "admin" : "recepcao";
            await run(() => setUserRole(token, user.id, role, user.version), `Perfil de ${user.display_name} alterado.`);
          }
        }}
      />
    </section>
  );
}
