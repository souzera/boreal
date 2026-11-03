import { zodResolver } from "@hookform/resolvers/zod";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { FormField, RequiredLegend } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  getPousada,
  getPousadaLogo,
  updatePousada,
  type ApiError,
  type LogoChange,
} from "@/lib/api";
import { useSessionStore } from "@/stores/session";

const MAX_LOGO_BYTES = 512 * 1024;
const LOGO_ERROR = "Escolha uma imagem PNG ou JPEG de até 512 KB.";

const schema = z.object({
  name: z.string().trim().min(1, "Informe o nome da pousada.").max(120, "No máximo 120 caracteres."),
  address: z.string().trim().max(255, "No máximo 255 caracteres."),
  phone: z.string().trim().max(30, "No máximo 30 caracteres."),
});
type Values = z.infer<typeof schema>;

function readAsBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result).split(",")[1] ?? "");
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
}

export default function PousadaForm({ onDirtyChange }: { onDirtyChange: (dirty: boolean) => void }) {
  const token = useSessionStore((s) => s.token) ?? "";
  const queryClient = useQueryClient();
  const { data } = useQuery({ queryKey: ["pousada"], queryFn: () => getPousada(token) });
  const { data: logo } = useQuery({
    queryKey: ["pousada-logo", data?.version],
    queryFn: () => getPousadaLogo(token),
    enabled: !!data?.has_logo,
  });
  const [logoChange, setLogoChange] = useState<LogoChange>({ kind: "keep" });
  const [preview, setPreview] = useState<string | null>(null);
  const [logoError, setLogoError] = useState<string | null>(null);
  const [message, setMessage] = useState<{ kind: "ok" | "error"; text: string } | null>(null);
  const {
    register,
    handleSubmit,
    reset,
    formState: { errors, isDirty, isSubmitting },
  } = useForm<Values>({
    resolver: zodResolver(schema),
    values: data ? { name: data.name, address: data.address, phone: data.phone } : undefined,
  });

  const dirty = isDirty || logoChange.kind !== "keep";
  useEffect(() => onDirtyChange(dirty), [dirty, onDirtyChange]);

  const shownLogo =
    logoChange.kind === "set"
      ? preview
      : logoChange.kind === "remove" || !logo
        ? null
        : `data:${logo.mime};base64,${logo.base64}`;

  const onFile = async (file: File | undefined) => {
    setLogoError(null);
    if (!file) return;
    if (!["image/png", "image/jpeg"].includes(file.type) || file.size > MAX_LOGO_BYTES) {
      setLogoError(LOGO_ERROR);
      return;
    }
    const base64 = await readAsBase64(file);
    setLogoChange({ kind: "set", mime: file.type, base64 });
    setPreview(`data:${file.type};base64,${base64}`);
  };

  const onSubmit = handleSubmit(async (values) => {
    setMessage(null);
    try {
      await updatePousada(token, { ...values, logo: logoChange }, data?.version ?? 0);
      await queryClient.invalidateQueries({ queryKey: ["pousada"] });
      reset(values);
      setLogoChange({ kind: "keep" });
      setPreview(null);
      setMessage({ kind: "ok", text: "Dados da pousada salvos." });
    } catch (e) {
      setMessage({ kind: "error", text: (e as ApiError).message });
    }
  });

  return (
    <Card>
      <CardHeader>
        <CardTitle>Dados da pousada</CardTitle>
        <p className="text-sm text-muted-foreground">Aparecem nos documentos impressos.</p>
      </CardHeader>
      <CardContent>
        <form onSubmit={onSubmit} noValidate className="grid gap-4 sm:grid-cols-2">
          <div className="sm:col-span-2">
            <FormField id="p-name" label="Nome da pousada" required error={errors.name?.message}>
              <Input id="p-name" {...register("name")} />
            </FormField>
          </div>
          <FormField id="p-address" label="Endereço" error={errors.address?.message}>
            <Input id="p-address" {...register("address")} />
          </FormField>
          <FormField id="p-phone" label="Telefone" error={errors.phone?.message}>
            <Input id="p-phone" {...register("phone")} />
          </FormField>
          <div className="grid gap-2 sm:col-span-2">
            <FormField id="p-logo" label="Logotipo (opcional)" error={logoError ?? undefined}>
              <Input
                id="p-logo"
                type="file"
                accept="image/png,image/jpeg"
                onChange={(e) => void onFile(e.target.files?.[0])}
              />
            </FormField>
            {shownLogo && (
              <div className="flex items-center gap-3">
                <img src={shownLogo} alt="Logotipo atual" className="h-16 w-auto rounded border" />
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    setLogoChange({ kind: "remove" });
                    setPreview(null);
                  }}
                >
                  Remover logotipo
                </Button>
              </div>
            )}
          </div>
          <div className="grid gap-3 sm:col-span-2">
            {message && (
              <p role={message.kind === "ok" ? "status" : "alert"} className={message.kind === "ok" ? "text-sm" : "text-sm text-destructive"}>
                {message.text}
              </p>
            )}
            <RequiredLegend />
            <Button type="submit" className="w-fit" disabled={isSubmitting || !data}>
              Salvar dados da pousada
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  );
}
