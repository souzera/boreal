import { zodResolver } from "@hookform/resolvers/zod";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { useNavigate } from "react-router-dom";
import { z } from "zod";

import { FormField, RequiredLegend } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { ApiError, login } from "@/lib/api";
import { useSessionStore } from "@/stores/session";

const schema = z.object({
  username: z.string().trim().min(1, "Informe o usuário."),
  password: z.string().min(1, "Informe a senha."),
});
type Values = z.infer<typeof schema>;

export default function Login() {
  const navigate = useNavigate();
  const setSession = useSessionStore((s) => s.setSession);
  const [formError, setFormError] = useState<string | null>(null);
  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<Values>({ resolver: zodResolver(schema) });

  const onSubmit = handleSubmit(async ({ username, password }) => {
    setFormError(null);
    try {
      const result = await login(username, password);
      setSession(result);
      navigate(result.must_change_password ? "/change-password" : "/", { replace: true });
    } catch (e) {
      const error = e as ApiError;
      setFormError(error.message.replace(/\.$/, ""));
    }
  });

  return (
    <main className="grid min-h-screen place-items-center bg-muted/40 p-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle>Boreal PMS</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={onSubmit} noValidate className="grid gap-4">
            <FormField id="username" label="Usuário" required error={errors.username?.message}>
              <Input id="username" autoComplete="username" autoFocus {...register("username")} />
            </FormField>
            <FormField id="password" label="Senha" required error={errors.password?.message}>
              <Input
                id="password"
                type="password"
                autoComplete="current-password"
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
              {isSubmitting ? "Entrando…" : "Entrar"}
            </Button>
          </form>
        </CardContent>
      </Card>
    </main>
  );
}
