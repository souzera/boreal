import { zodResolver } from "@hookform/resolvers/zod";
import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { FormField, RequiredLegend } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { bootstrapAdmin, type ApiError } from "@/lib/api";

const schema = z
  .object({
    username: z.string().trim().min(1, "Informe o usuário."),
    displayName: z.string().trim().min(1, "Informe o nome completo."),
    password: z.string().min(8, "A senha deve ter pelo menos 8 caracteres."),
    confirm: z.string().min(1, "Confirme a senha."),
  })
  .refine((v) => v.password === v.confirm, { path: ["confirm"], message: "As senhas não conferem." });
type Values = z.infer<typeof schema>;

/** Primeira execução: cria o administrador inicial. */
export default function FirstRun() {
  const queryClient = useQueryClient();
  const [formError, setFormError] = useState<string | null>(null);
  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<Values>({ resolver: zodResolver(schema) });

  const onSubmit = handleSubmit(async ({ username, displayName, password }) => {
    setFormError(null);
    try {
      await bootstrapAdmin(username, displayName, password);
      await queryClient.invalidateQueries({ queryKey: ["startup"] });
    } catch (e) {
      setFormError((e as ApiError).message);
    }
  });

  return (
    <main className="grid min-h-screen place-items-center bg-muted/40 p-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle>Configuração inicial</CardTitle>
          <p className="text-sm text-muted-foreground">Crie o usuário administrador do sistema.</p>
        </CardHeader>
        <CardContent>
          <form onSubmit={onSubmit} noValidate className="grid gap-4">
            <FormField id="fr-username" label="Usuário" required error={errors.username?.message}>
              <Input id="fr-username" autoComplete="username" {...register("username")} />
            </FormField>
            <FormField id="fr-name" label="Nome completo" required error={errors.displayName?.message}>
              <Input id="fr-name" {...register("displayName")} />
            </FormField>
            <FormField id="fr-password" label="Senha" required error={errors.password?.message}>
              <Input id="fr-password" type="password" autoComplete="new-password" {...register("password")} />
            </FormField>
            <FormField id="fr-confirm" label="Confirmar senha" required error={errors.confirm?.message}>
              <Input id="fr-confirm" type="password" autoComplete="new-password" {...register("confirm")} />
            </FormField>
            {formError && (
              <p role="alert" className="text-sm text-destructive">
                {formError}
              </p>
            )}
            <RequiredLegend />
            <Button type="submit" disabled={isSubmitting}>
              Criar administrador
            </Button>
          </form>
        </CardContent>
      </Card>
    </main>
  );
}
