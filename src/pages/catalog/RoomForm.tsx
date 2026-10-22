import { zodResolver } from "@hookform/resolvers/zod";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { FormField, RequiredLegend } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import type { ApiError, Category, Room, RoomInput } from "@/lib/api";

const schema = z.object({
  code: z.string().trim().min(1, "Informe o código do quarto.").max(20, "No máximo 20 caracteres."),
  category_id: z.number({ error: "Escolha a categoria." }).int().min(1, "Escolha a categoria."),
  max_capacity: z
    .number({ error: "Informe a capacidade." })
    .int("Informe um número inteiro.")
    .min(1, "Capacidade máxima: informe de 1 a 50 pessoas.")
    .max(50, "Capacidade máxima: informe de 1 a 50 pessoas."),
  description: z.string().trim().max(255, "No máximo 255 caracteres."),
  notes: z.string().trim().max(1000, "No máximo 1000 caracteres."),
});
type Values = z.infer<typeof schema>;

const empty = { code: "", category_id: 0, max_capacity: 2, description: "", notes: "" };

interface Props {
  open: boolean;
  /** Quarto em edição; `null` cria um novo. */
  room: Room | null;
  categories: Category[];
  onClose: () => void;
  onSubmit: (input: RoomInput) => Promise<void>;
}

export default function RoomForm({ open, room, categories, onClose, onSubmit }: Props) {
  const [formError, setFormError] = useState<string | null>(null);
  const {
    register,
    handleSubmit,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<Values>({ resolver: zodResolver(schema), defaultValues: empty });

  useEffect(() => {
    if (open) {
      setFormError(null);
      reset(
        room
          ? {
              code: room.code,
              category_id: room.category_id,
              max_capacity: room.max_capacity,
              description: room.description,
              notes: room.notes,
            }
          : empty,
      );
    }
  }, [open, room, reset]);

  // Para novos quartos só categorias ativas; na edição, mantém a categoria atual mesmo se desativada.
  const options = categories.filter((c) => c.active || c.id === room?.category_id);

  const submit = handleSubmit(async (values) => {
    setFormError(null);
    try {
      await onSubmit(values);
    } catch (e) {
      setFormError((e as ApiError).message);
    }
  });

  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>{room ? `Editar quarto ${room.code}` : "Novo quarto"}</DialogTitle>
        </DialogHeader>
        <form onSubmit={submit} noValidate className="grid gap-4">
          <FormField id="room-code" label="Código do quarto" required error={errors.code?.message}>
            <Input id="room-code" {...register("code")} />
          </FormField>
          <FormField id="room-category" label="Categoria" required error={errors.category_id?.message}>
            <select
              id="room-category"
              className="h-8 rounded-lg border border-input bg-transparent px-2 text-sm"
              {...register("category_id", { valueAsNumber: true })}
            >
              <option value={0}>Escolha…</option>
              {options.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </select>
          </FormField>
          <FormField id="room-capacity" label="Capacidade máxima (pessoas)" required error={errors.max_capacity?.message}>
            <Input id="room-capacity" type="number" {...register("max_capacity", { valueAsNumber: true })} />
          </FormField>
          <FormField id="room-description" label="Descrição" error={errors.description?.message}>
            <Input id="room-description" {...register("description")} />
          </FormField>
          <FormField id="room-notes" label="Observações" error={errors.notes?.message}>
            <Textarea id="room-notes" {...register("notes")} />
          </FormField>
          {formError && (
            <p role="alert" className="text-sm text-destructive">
              {formError}
            </p>
          )}
          <RequiredLegend />
          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose}>
              Cancelar
            </Button>
            <Button type="submit" disabled={isSubmitting}>
              Salvar quarto
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
