import { zodResolver } from "@hookform/resolvers/zod";
import { useEffect, useState } from "react";
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
import { useGuardedClose } from "@/hooks/useGuardedClose";
import type { ApiError, UserRow } from "@/lib/api";

const schema = z.object({ password: z.string().min(6, "A senha deve ter pelo menos 6 caracteres.") });
type Values = z.infer<typeof schema>;

interface Props {
  user: UserRow | null;
  onClose: () => void;
  onSubmit: (user: UserRow, tempPassword: string) => Promise<void>;
}

export default function ResetPasswordDialog({ user, onClose, onSubmit }: Props) {
  const [formError, setFormError] = useState<string | null>(null);
  const {
    register,
    handleSubmit,
    reset,
    formState: { errors, isDirty, isSubmitting },
  } = useForm<Values>({ resolver: zodResolver(schema) });
  const { requestClose, confirm } = useGuardedClose(isDirty, () => {
    reset();
    onClose();
  });

  // Cada abertura começa sem senha digitada.
  useEffect(() => {
    reset();
    setFormError(null);
  }, [user, reset]);

  const submit = handleSubmit(async ({ password }) => {
    if (!user) return;
    setFormError(null);
    try {
      await onSubmit(user, password);
      reset();
    } catch (e) {
      setFormError((e as ApiError).message);
    }
  });

  return (
    <>
    <Dialog open={user !== null} onOpenChange={(next) => !next && requestClose()}>
      <DialogContent showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>Redefinir senha de {user?.display_name}</DialogTitle>
          <DialogDescription>
            As sessões abertas dessa pessoa serão encerradas e ela precisará trocar a senha ao entrar.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={submit} noValidate className="grid gap-4">
          <FormField id="r-password" label="Senha temporária" required error={errors.password?.message}>
            <Input id="r-password" type="password" autoComplete="new-password" {...register("password")} />
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
              Redefinir senha
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
    {confirm}
    </>
  );
}
