import { zodResolver } from "@hookform/resolvers/zod";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { useNavigate } from "react-router-dom";
import { z } from "zod";

import { FormField, RequiredLegend } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { changeOwnPassword, type ApiError } from "@/lib/api";
import { useSessionStore } from "@/stores/session";

const schema = z
  .object({
    current: z.string().min(1, "Informe a senha atual."),
    next: z.string().min(6, "A senha deve ter pelo menos 6 caracteres."),
    confirm: z.string().min(1, "Confirme a nova senha."),
  })
  .refine((v) => v.next === v.confirm, { path: ["confirm"], message: "As senhas não conferem." });
type Values = z.infer<typeof schema>;

export default function ChangePassword() {
  const navigate = useNavigate();
  const { token, mustChangePassword, setMustChangePassword } = useSessionStore();
  const [formError, setFormError] = useState<string | null>(null);
  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<Values>({ resolver: zodResolver(schema) });

  const onSubmit = handleSubmit(async ({ current, next }) => {
    setFormError(null);
    try {
      await changeOwnPassword(token ?? "", current, next);
      setMustChangePassword(false);
      navigate("/", { replace: true });
    } catch (e) {
      setFormError((e as ApiError).message);
    }
  });

  return (
    <main className="grid min-h-screen place-items-center bg-muted/40 p-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle>Alterar senha</CardTitle>
          {mustChangePassword && (
            <p className="text-sm text-muted-foreground">
              Por segurança, defina uma nova senha antes de continuar.
            </p>
          )}
        </CardHeader>
        <CardContent>
          <form onSubmit={onSubmit} noValidate className="grid gap-4">
            <FormField id="current" label="Senha atual" required error={errors.current?.message}>
              <Input id="current" type="password" autoComplete="current-password" {...register("current")} />
            </FormField>
            <FormField id="next" label="Nova senha" required error={errors.next?.message}>
              <Input id="next" type="password" autoComplete="new-password" {...register("next")} />
            </FormField>
            <FormField id="confirm" label="Confirmar nova senha" required error={errors.confirm?.message}>
              <Input id="confirm" type="password" autoComplete="new-password" {...register("confirm")} />
            </FormField>
            {formError && (
              <p role="alert" className="text-sm text-destructive">
                {formError}
              </p>
            )}
            <RequiredLegend />
            <Button type="submit" disabled={isSubmitting}>
              Salvar nova senha
            </Button>
          </form>
        </CardContent>
      </Card>
    </main>
  );
}
