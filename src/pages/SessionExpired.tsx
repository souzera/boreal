import { zodResolver } from "@hookform/resolvers/zod";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { FormField, RequiredLegend } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { login, type ApiError } from "@/lib/api";
import { useSessionStore } from "@/stores/session";

const schema = z.object({ password: z.string().min(1, "Informe a senha.") });
type Values = z.infer<typeof schema>;

/**
 * O backend encerrou a sessão. A tela de trás continua montada (oculta), então a pessoa
 * pode entrar de novo e seguir de onde parou, sem perder o que estava digitando.
 */
export default function SessionExpired() {
  const { user, setSession, clear } = useSessionStore();
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
      setSession(await login(user?.username ?? "", password));
      reset();
    } catch (e) {
      setFormError((e as ApiError).message);
    }
  });

  return (
    <main className="grid min-h-screen place-items-center bg-muted/40 p-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle>Sessão encerrada</CardTitle>
          <p className="text-sm text-muted-foreground">
            Sua sessão foi encerrada (por exemplo, por mudança de senha ou de permissões). Entre novamente
            para continuar de onde parou; o que você digitou foi preservado.
          </p>
        </CardHeader>
        <CardContent>
          <form onSubmit={onSubmit} noValidate className="grid gap-4">
            <FormField id="expired-user" label="Usuário" required>
              <Input id="expired-user" value={user?.username ?? ""} readOnly />
            </FormField>
            <FormField id="expired-password" label="Senha" required error={errors.password?.message}>
              <Input
                id="expired-password"
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
              Entrar novamente
            </Button>
            <Button type="button" variant="ghost" onClick={clear}>
              Sair sem salvar
            </Button>
          </form>
        </CardContent>
      </Card>
    </main>
  );
}
