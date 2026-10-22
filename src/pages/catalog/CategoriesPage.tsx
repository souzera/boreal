import { zodResolver } from "@hookform/resolvers/zod";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { Link } from "react-router-dom";
import { z } from "zod";

import { ConfirmDialog } from "@/components/ConfirmDialog";
import { FormField, RequiredLegend } from "@/components/FormField";
import { PageHeader } from "@/components/PageHeader";
import { useGuardedClose } from "@/hooks/useGuardedClose";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import {
  createCategory,
  listCategories,
  setCategoryActive,
  updateCategory,
  type ApiError,
  type Category,
} from "@/lib/api";
import { useSessionStore } from "@/stores/session";

const schema = z.object({
  name: z.string().trim().min(1, "Informe o nome da categoria.").max(80, "No máximo 80 caracteres."),
  description: z.string().trim().max(255, "No máximo 255 caracteres."),
});
type Values = z.infer<typeof schema>;

function CategoryForm({
  category,
  open,
  onClose,
  onSubmit,
}: {
  category: Category | null;
  open: boolean;
  onClose: () => void;
  onSubmit: (values: Values) => Promise<void>;
}) {
  const [formError, setFormError] = useState<string | null>(null);
  const {
    register,
    handleSubmit,
    reset,
    formState: { errors, isDirty, isSubmitting },
  } = useForm<Values>({ resolver: zodResolver(schema), defaultValues: { name: "", description: "" } });
  const { requestClose, confirm } = useGuardedClose(isDirty, onClose);

  useEffect(() => {
    if (open) {
      setFormError(null);
      reset(category ? { name: category.name, description: category.description } : { name: "", description: "" });
    }
  }, [open, category, reset]);

  const submit = handleSubmit(async (values) => {
    setFormError(null);
    try {
      await onSubmit(values);
    } catch (e) {
      setFormError((e as ApiError).message);
    }
  });

  return (
    <>
    <Dialog open={open} onOpenChange={(next) => !next && requestClose()}>
      <DialogContent showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>{category ? `Editar categoria ${category.name}` : "Nova categoria"}</DialogTitle>
        </DialogHeader>
        <form onSubmit={submit} noValidate className="grid gap-4">
          <FormField id="cat-name" label="Nome da categoria" required error={errors.name?.message}>
            <Input id="cat-name" {...register("name")} />
          </FormField>
          <FormField id="cat-description" label="Descrição" error={errors.description?.message}>
            <Input id="cat-description" {...register("description")} />
          </FormField>
          {formError && (
            <p role="alert" className="text-sm text-destructive">
              {formError}
            </p>
          )}
          <RequiredLegend />
          <DialogFooter>
            <Button type="button" variant="outline" onClick={requestClose}>
              Cancelar
            </Button>
            <Button type="submit" disabled={isSubmitting}>
              Salvar categoria
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
    {confirm}
    </>
  );
}

export default function CategoriesPage() {
  const token = useSessionStore((s) => s.token) ?? "";
  const canManage = useSessionStore((s) => s.user?.role === "admin");
  const queryClient = useQueryClient();
  const { data = [] } = useQuery({ queryKey: ["categories", "all"], queryFn: () => listCategories(token, false) });
  const [form, setForm] = useState<{ open: boolean; category: Category | null }>({ open: false, category: null });
  const [toDeactivate, setToDeactivate] = useState<Category | null>(null);
  const [notice, setNotice] = useState<{ kind: "ok" | "error"; text: string } | null>(null);
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["categories"] });

  const save = async (values: Values) => {
    const editing = form.category;
    if (editing) await updateCategory(token, editing.id, values, editing.version);
    else await createCategory(token, values);
    await refresh();
    setForm({ open: false, category: null });
    setNotice({
      kind: "ok",
      text: editing ? `Categoria ${values.name} atualizada.` : `Categoria ${values.name} criada.`,
    });
  };

  const toggle = async (category: Category, active: boolean) => {
    setNotice(null);
    try {
      await setCategoryActive(token, category.id, active);
      await refresh();
      setNotice({ kind: "ok", text: `Categoria ${category.name} ${active ? "reativada" : "desativada"}.` });
    } catch (e) {
      setNotice({ kind: "error", text: (e as ApiError).message });
    }
  };

  return (
    <section>
      <PageHeader
        title="Categorias de quartos"
        description="Agrupam os quartos por tipo."
        action={
          <div className="flex gap-2">
            <Button variant="outline" render={<Link to="/quartos" />}>
              Voltar aos quartos
            </Button>
            {canManage && <Button onClick={() => setForm({ open: true, category: null })}>Nova categoria</Button>}
          </div>
        }
      />
      {notice && (
        <p role={notice.kind === "ok" ? "status" : "alert"} className={notice.kind === "ok" ? "mb-3 text-sm" : "mb-3 text-sm text-destructive"}>
          {notice.text}
        </p>
      )}
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Nome</TableHead>
            <TableHead>Descrição</TableHead>
            <TableHead>Situação</TableHead>
            {canManage && <TableHead>Ações</TableHead>}
          </TableRow>
        </TableHeader>
        <TableBody>
          {data.length === 0 ? (
            <TableRow>
              <TableCell colSpan={4} className="py-8 text-center text-muted-foreground">
                Nenhuma categoria cadastrada.
              </TableCell>
            </TableRow>
          ) : (
            data.map((c) => (
              <TableRow key={c.id}>
                <TableCell>{c.name}</TableCell>
                <TableCell>{c.description || "—"}</TableCell>
                <TableCell>{c.active ? "Ativa" : "Inativa"}</TableCell>
                {canManage && (
                  <TableCell>
                    <div className="flex gap-1">
                      <Button size="sm" variant="outline" onClick={() => setForm({ open: true, category: c })}>
                        Editar
                      </Button>
                      {c.active ? (
                        <Button size="sm" variant="outline" onClick={() => setToDeactivate(c)}>
                          Desativar
                        </Button>
                      ) : (
                        <Button size="sm" variant="outline" onClick={() => toggle(c, true)}>
                          Reativar
                        </Button>
                      )}
                    </div>
                  </TableCell>
                )}
              </TableRow>
            ))
          )}
        </TableBody>
      </Table>
      <CategoryForm
        open={form.open}
        category={form.category}
        onClose={() => setForm({ open: false, category: null })}
        onSubmit={save}
      />
      <ConfirmDialog
        open={toDeactivate !== null}
        title="Desativar categoria?"
        description={`A categoria ${toDeactivate?.name} deixa de ser oferecida para novos quartos.`}
        confirmLabel="Desativar categoria"
        destructive
        onCancel={() => setToDeactivate(null)}
        onConfirm={async () => {
          const category = toDeactivate;
          setToDeactivate(null);
          if (category) await toggle(category, false);
        }}
      />
    </section>
  );
}
