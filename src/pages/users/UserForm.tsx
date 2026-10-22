import { zodResolver } from "@hookform/resolvers/zod";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { FormField, RequiredLegend } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import type { ApiError, NewUserInput } from "@/lib/api";
import type { Role } from "@/stores/session";
import { roleLabel } from "./labels";

const schema = z.object({
  username: z.string().trim().min(1, "Informe o usuário."),
  display_name: z.string().trim().min(1, "Informe o nome completo."),
  role: z.enum(["admin", "recepcao"]),
  initial_password: z.string().min(6, "A senha deve ter pelo menos 6 caracteres."),
});
type Values = z.infer<typeof schema>;

interface Props {
  open: boolean;
  onClose: () => void;
  onSubmit: (input: NewUserInput) => Promise<void>;
}

export default function UserForm({ open, onClose, onSubmit }: Props) {
  const [formError, setFormError] = useState<string | null>(null);
  const {
    register,
    handleSubmit,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<Values>({ resolver: zodResolver(schema), defaultValues: { role: "recepcao" } });

  const submit = handleSubmit(async (values) => {
    setFormError(null);
    try {
      await onSubmit(values);
      reset({ role: "recepcao" });
    } catch (e) {
      setFormError((e as ApiError).message);
    }
  });

  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>Novo usuário</DialogTitle>
          <DialogDescription>
            A senha temporária deve ser trocada pela pessoa no primeiro acesso.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={submit} noValidate className="grid gap-4">
          <FormField id="u-username" label="Usuário" required error={errors.username?.message}>
            <Input id="u-username" autoComplete="off" {...register("username")} />
          </FormField>
          <FormField id="u-name" label="Nome completo" required error={errors.display_name?.message}>
            <Input id="u-name" {...register("display_name")} />
          </FormField>
          <FormField id="u-role" label="Perfil" required>
            <select
              id="u-role"
              className="h-8 rounded-lg border border-input bg-transparent px-2 text-sm"
              {...register("role")}
            >
              {(Object.keys(roleLabel) as Role[]).map((role) => (
                <option key={role} value={role}>
                  {roleLabel[role]}
                </option>
              ))}
            </select>
          </FormField>
          <FormField id="u-password" label="Senha temporária" required error={errors.initial_password?.message}>
            <Input id="u-password" type="password" autoComplete="new-password" {...register("initial_password")} />
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
              Criar usuário
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
