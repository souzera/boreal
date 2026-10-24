import { zodResolver } from "@hookform/resolvers/zod";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { ConfirmDialog } from "@/components/ConfirmDialog";
import { FormField, RequiredLegend } from "@/components/FormField";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useGuardedClose } from "@/hooks/useGuardedClose";
import { createItem, listItems, setItemActive, updateItem, type ApiError, type Item } from "@/lib/api";
import { formatMoney, parseMoney } from "@/lib/format";
import { useSessionStore } from "@/stores/session";

const schema = z.object({
  name: z.string().trim().min(1, "Informe o nome do item.").max(80, "No máximo 80 caracteres."),
  price: z.string().refine((v) => (parseMoney(v) ?? 0) > 0, "Informe um preço maior que zero, por exemplo R$ 6,00."),
});
type Values = z.infer<typeof schema>;

const empty: Values = { name: "", price: "" };

function priceText(cents: number): string {
  return formatMoney(cents);
}

function ItemForm({
  item,
  open,
  onClose,
  onSubmit,
}: {
  item: Item | null;
  open: boolean;
  onClose: () => void;
  onSubmit: (values: { name: string; default_price_cents: number }) => Promise<void>;
}) {
  const [formError, setFormError] = useState<string | null>(null);
  const {
    register,
    handleSubmit,
    reset,
    formState: { errors, isDirty, isSubmitting },
  } = useForm<Values>({ resolver: zodResolver(schema), defaultValues: empty });
  const { requestClose, confirm } = useGuardedClose(isDirty, onClose);

  useEffect(() => {
    if (open) {
      setFormError(null);
      reset(item ? { name: item.name, price: priceText(item.default_price_cents) } : empty);
    }
  }, [open, item, reset]);

  const submit = handleSubmit(async (values) => {
    setFormError(null);
    try {
      await onSubmit({ name: values.name, default_price_cents: parseMoney(values.price) ?? 0 });
    } catch (e) {
      setFormError((e as ApiError).message);
    }
  });

  return (
    <>
      <Dialog open={open} onOpenChange={(next) => !next && requestClose()}>
        <DialogContent showCloseButton={false}>
          <DialogHeader>
            <DialogTitle>{item ? `Editar item ${item.name}` : "Novo item"}</DialogTitle>
          </DialogHeader>
          <form onSubmit={submit} noValidate className="grid gap-4">
            <FormField id="item-name" label="Nome do item" required error={errors.name?.message}>
              <Input id="item-name" {...register("name")} />
            </FormField>
            <FormField id="item-price" label="Preço padrão" required error={errors.price?.message}>
              <Input id="item-price" inputMode="decimal" placeholder="R$ 0,00" {...register("price")} />
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
                Salvar item
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
      {confirm}
    </>
  );
}

export default function ItemsPage() {
  const token = useSessionStore((s) => s.token) ?? "";
  const canManage = useSessionStore((s) => s.user?.role === "admin");
  const queryClient = useQueryClient();
  const { data = [] } = useQuery({ queryKey: ["items", "all"], queryFn: () => listItems(token, false) });
  const [form, setForm] = useState<{ open: boolean; item: Item | null }>({ open: false, item: null });
  const [toDeactivate, setToDeactivate] = useState<Item | null>(null);
  const [notice, setNotice] = useState<{ kind: "ok" | "error"; text: string } | null>(null);
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["items"] });

  const save = async (values: { name: string; default_price_cents: number }) => {
    const editing = form.item;
    if (editing) await updateItem(token, editing.id, values, editing.version);
    else await createItem(token, values);
    await refresh();
    setForm({ open: false, item: null });
    setNotice({ kind: "ok", text: editing ? `Item ${values.name} atualizado.` : `Item ${values.name} criado.` });
  };

  const toggle = async (item: Item, active: boolean) => {
    setNotice(null);
    try {
      await setItemActive(token, item.id, active, item.version);
      await refresh();
      setNotice({ kind: "ok", text: `Item ${item.name} ${active ? "reativado" : "desativado"}.` });
    } catch (e) {
      setNotice({ kind: "error", text: (e as ApiError).message });
    }
  };

  return (
    <section>
      <PageHeader
        title="Itens e serviços"
        description="Lista de consumos e serviços para lançar na conta do hóspede. Não controla estoque."
        action={canManage ? <Button onClick={() => setForm({ open: true, item: null })}>Novo item</Button> : undefined}
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
            <TableHead>Preço padrão</TableHead>
            <TableHead>Situação</TableHead>
            {canManage && <TableHead>Ações</TableHead>}
          </TableRow>
        </TableHeader>
        <TableBody>
          {data.length === 0 ? (
            <TableRow>
              <TableCell colSpan={4} className="py-8 text-center text-muted-foreground">
                Nenhum item cadastrado.
              </TableCell>
            </TableRow>
          ) : (
            data.map((i) => (
              <TableRow key={i.id}>
                <TableCell>{i.name}</TableCell>
                <TableCell>{formatMoney(i.default_price_cents)}</TableCell>
                <TableCell>{i.active ? "Ativo" : "Inativo"}</TableCell>
                {canManage && (
                  <TableCell>
                    <div className="flex gap-1">
                      <Button size="sm" variant="outline" onClick={() => setForm({ open: true, item: i })}>
                        Editar
                      </Button>
                      {i.active ? (
                        <Button size="sm" variant="outline" onClick={() => setToDeactivate(i)}>
                          Desativar
                        </Button>
                      ) : (
                        <Button size="sm" variant="outline" onClick={() => toggle(i, true)}>
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
      <ItemForm open={form.open} item={form.item} onClose={() => setForm({ open: false, item: null })} onSubmit={save} />
      <ConfirmDialog
        open={toDeactivate !== null}
        title="Desativar item?"
        description={`O item ${toDeactivate?.name} deixa de ser oferecido em novos lançamentos. Lançamentos antigos continuam como estão.`}
        confirmLabel="Desativar item"
        destructive
        onCancel={() => setToDeactivate(null)}
        onConfirm={async () => {
          const item = toDeactivate;
          setToDeactivate(null);
          if (item) await toggle(item, false);
        }}
      />
    </section>
  );
}
