import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
  createExpenseCategory,
  renameExpenseCategory,
  setExpenseCategoryActive,
  type ApiError,
  type ExpenseCategory,
} from "@/lib/api";
import { useSessionStore } from "@/stores/session";

interface Props {
  categories: ExpenseCategory[];
}

/** Categorias de despesa: o administrador cria, renomeia e desativa (nunca apaga o que tem histórico). */
export default function CategoriesPanel({ categories }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const queryClient = useQueryClient();
  const [editing, setEditing] = useState<{ category: ExpenseCategory | null } | null>(null);
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    if (editing) {
      setName(editing.category?.name ?? "");
      setError(null);
    }
  }, [editing]);

  const refresh = () => queryClient.invalidateQueries({ queryKey: ["expense-categories"] });

  const save = async () => {
    if (name.trim() === "") {
      setError("Informe o nome da categoria.");
      return;
    }
    try {
      if (editing?.category) await renameExpenseCategory(token, editing.category.id, name.trim(), editing.category.version);
      else await createExpenseCategory(token, name.trim());
      await refresh();
      setNotice(editing?.category ? "Categoria renomeada." : "Categoria criada.");
      setEditing(null);
    } catch (e) {
      setError((e as ApiError).message);
    }
  };

  const toggle = async (category: ExpenseCategory) => {
    setNotice(null);
    try {
      await setExpenseCategoryActive(token, category.id, !category.active, category.version);
      await refresh();
      setNotice(category.active ? `Categoria ${category.name} desativada.` : `Categoria ${category.name} reativada.`);
    } catch (e) {
      setError((e as ApiError).message);
    }
  };

  return (
    <section aria-label="Categorias de despesa">
      <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-base font-semibold">Categorias de despesa</h2>
        <Button size="sm" variant="outline" onClick={() => setEditing({ category: null })}>
          Nova categoria
        </Button>
      </div>
      {notice && (
        <p role="status" className="mb-2 text-sm">
          {notice}
        </p>
      )}
      <ul className="grid gap-1 text-sm">
        {categories.map((c) => (
          <li key={c.id} className="flex flex-wrap items-center gap-2">
            <span>{c.name}</span>
            {!c.active && <span className="text-muted-foreground">(inativa)</span>}
            <Button size="sm" variant="outline" onClick={() => setEditing({ category: c })}>
              Renomear
            </Button>
            <Button size="sm" variant="outline" onClick={() => void toggle(c)}>
              {c.active ? "Desativar" : "Reativar"}
            </Button>
          </li>
        ))}
      </ul>
      <Dialog open={editing !== null} onOpenChange={(next) => !next && setEditing(null)}>
        <DialogContent showCloseButton={false}>
          <DialogHeader>
            <DialogTitle>{editing?.category ? `Renomear categoria ${editing.category.name}` : "Nova categoria"}</DialogTitle>
          </DialogHeader>
          <FormField id="cat-name" label="Nome da categoria" required error={error ?? undefined}>
            <Input id="cat-name" value={name} onChange={(e) => setName(e.target.value)} />
          </FormField>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setEditing(null)}>
              Cancelar
            </Button>
            <Button type="button" onClick={() => void save()}>
              Salvar categoria
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  );
}
