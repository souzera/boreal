import { zodResolver } from "@hookform/resolvers/zod";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { FormField, RequiredLegend } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { unlockSession, type ApiError } from "@/lib/api";
import { useSessionStore } from "@/stores/session";

const schema = z.object({ password: z.string().min(1, "Informe a senha.") });
type Values = z.infer<typeof schema>;

/** Cobre a tela inteira; nenhum dado de hóspede ou financeiro é renderizado por trás. */
export default function LockScreen() {
  const { token, user, unlock, clear } = useSessionStore();
  const [formError, setFormError] = useState<string | null>(null);
  const {
    register,
    handleSubmit,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<Values>({ resolver: zodResolver(schema) });

  const onSubmit = handleSubmit(async ({ password }) => {
    setFormError(null);
    try {
      await unlockSession(token ?? "", password);
      reset();
      unlock();
    } catch (e) {
      setFormError((e as ApiError).message);
    }
  });

  return (
    <main className="grid min-h-screen place-items-center bg-muted/40 p-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle>Sessão bloqueada</CardTitle>
          <p className="text-sm text-muted-foreground">
            {user?.display_name}, informe sua senha para continuar.
          </p>
        </CardHeader>
        <CardContent>
          <form onSubmit={onSubmit} noValidate className="grid gap-4">
            <FormField id="unlock-password" label="Senha" required error={errors.password?.message}>
              <Input
                id="unlock-password"
                type="password"
                autoComplete="current-password"
                autoFocus
                {...register("password")}
              />
            </FormField>
            {formError && (
              <p role="alert" className="text-sm text-destructive">
                {formError}
              </p>
            )}
            <RequiredLegend />
            <Button type="submit" disabled={isSubmitting}>
              Desbloquear
            </Button>
            <Button type="button" variant="ghost" onClick={clear}>
              Sair
            </Button>
          </form>
        </CardContent>
      </Card>
    </main>
  );
}
