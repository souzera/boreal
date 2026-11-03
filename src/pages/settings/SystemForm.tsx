import { zodResolver } from "@hookform/resolvers/zod";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { FormField, RequiredLegend } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { getSettings, updateSettings, type ApiError } from "@/lib/api";
import { useSessionStore } from "@/stores/session";

const whole = (min: number, max: number, unit: string) =>
  z
    .number({ error: "Informe um número." })
    .int("Informe um número inteiro.")
    .min(min, `Informe de ${min} a ${max} ${unit}.`)
    .max(max, `Informe de ${min} a ${max} ${unit}.`);

const time = z.string().regex(/^([01]\d|2[0-3]):[0-5]\d$/, "Use o formato HH:MM, por exemplo 14:00.");

const schema = z.object({
  max_failed_attempts: whole(3, 20, "tentativas"),
  lockout_minutes: whole(1, 1440, "minutos"),
  idle_lock_minutes: whole(1, 480, "minutos"),
  default_check_in_time: time,
  default_check_out_time: time,
  late_checkout_tolerance_minutes: whole(0, 600, "minutos"),
  no_show_tolerance_minutes: whole(0, 600, "minutos"),
  cleaning_interval_minutes: whole(0, 600, "minutos"),
  deposit_percent: z
    .number({ error: "Informe um número." })
    .min(0, "Informe de 0 a 100%.")
    .max(100, "Informe de 0 a 100%."),
  deposit_due_days_after_booking: whole(0, 60, "dias"),
  deposit_due_days_before_arrival: whole(0, 60, "dias"),
  late_fee_percent: z
    .number({ error: "Informe um número." })
    .min(0, "Informe de 0 a 100%.")
    .max(100, "Informe de 0 a 100%."),
});
type Values = z.infer<typeof schema>;

const number = { valueAsNumber: true } as const;

export default function SystemForm({ onDirtyChange }: { onDirtyChange: (dirty: boolean) => void }) {
  const token = useSessionStore((s) => s.token) ?? "";
  const queryClient = useQueryClient();
  const { data } = useQuery({ queryKey: ["settings"], queryFn: () => getSettings(token) });
  // Sinal e cobrança de saída tardia são guardados em pontos-base (3000 = 30%) e mostrados em percentual.
  const formValues = useMemo<Values | undefined>(() => {
    if (!data) return undefined;
    const { deposit_percent_bp, late_checkout_fee_bp, ...rest } = data.settings;
    return { ...rest, deposit_percent: deposit_percent_bp / 100, late_fee_percent: late_checkout_fee_bp / 100 };
  }, [data]);
  const [message, setMessage] = useState<{ kind: "ok" | "error"; text: string; conflict?: boolean } | null>(null);
  const {
    register,
    handleSubmit,
    reset,
    formState: { errors, isDirty, isSubmitting },
  } = useForm<Values>({ resolver: zodResolver(schema), values: formValues });

  useEffect(() => onDirtyChange(isDirty), [isDirty, onDirtyChange]);

  const onSubmit = handleSubmit(async (values) => {
    setMessage(null);
    try {
      const { deposit_percent, late_fee_percent, ...rest } = values;
      await updateSettings(
        token,
        { ...rest, deposit_percent_bp: Math.round(deposit_percent * 100), late_checkout_fee_bp: Math.round(late_fee_percent * 100) },
        data?.version ?? 0,
      );
      await queryClient.invalidateQueries({ queryKey: ["settings"] });
      reset(values);
      // O backend já aplica o novo limite; a interface passa a usá-lo na hora.
      useSessionStore.getState().setIdleMinutes(values.idle_lock_minutes);
      setMessage({ kind: "ok", text: "Configurações salvas." });
    } catch (e) {
      const error = e as ApiError;
      setMessage({ kind: "error", text: error.message, conflict: error.code === "conflict" });
    }
  });

  const reload = async () => {
    setMessage(null);
    await queryClient.invalidateQueries({ queryKey: ["settings"] });
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Configurações do sistema</CardTitle>
        <p className="text-sm text-muted-foreground">
          Os valores iniciais são provisórios e podem ser ajustados à rotina da pousada.
        </p>
      </CardHeader>
      <CardContent>
        <form onSubmit={onSubmit} noValidate className="grid gap-4 sm:grid-cols-2">
          <FormField id="s-attempts" label="Tentativas inválidas antes do bloqueio da conta" required error={errors.max_failed_attempts?.message}>
            <Input id="s-attempts" type="number" {...register("max_failed_attempts", number)} />
          </FormField>
          <FormField id="s-lockout" label="Minutos de bloqueio da conta" required error={errors.lockout_minutes?.message}>
            <Input id="s-lockout" type="number" {...register("lockout_minutes", number)} />
          </FormField>
          <FormField id="s-idle" label="Minutos de inatividade até bloquear a sessão" required error={errors.idle_lock_minutes?.message}>
            <Input id="s-idle" type="number" {...register("idle_lock_minutes", number)} />
          </FormField>
          <FormField id="s-checkin" label="Horário padrão de check-in" required error={errors.default_check_in_time?.message}>
            <Input id="s-checkin" placeholder="14:00" {...register("default_check_in_time")} />
          </FormField>
          <FormField id="s-checkout" label="Horário padrão de check-out" required error={errors.default_check_out_time?.message}>
            <Input id="s-checkout" placeholder="12:00" {...register("default_check_out_time")} />
          </FormField>
          <FormField id="s-late" label="Tolerância de saída tardia (minutos)" required error={errors.late_checkout_tolerance_minutes?.message}>
            <Input id="s-late" type="number" {...register("late_checkout_tolerance_minutes", number)} />
          </FormField>
          <FormField id="s-late-fee" label="Cobrança de saída tardia (% da última diária) — provisório" required error={errors.late_fee_percent?.message}>
            <Input id="s-late-fee" type="number" step="0.01" {...register("late_fee_percent", number)} />
          </FormField>
          <FormField id="s-noshow" label="Tolerância de no-show (minutos)" required error={errors.no_show_tolerance_minutes?.message}>
            <Input id="s-noshow" type="number" {...register("no_show_tolerance_minutes", number)} />
          </FormField>
          <FormField id="s-cleaning" label="Intervalo de limpeza entre hospedagens (minutos)" required error={errors.cleaning_interval_minutes?.message}>
            <Input id="s-cleaning" type="number" {...register("cleaning_interval_minutes", number)} />
          </FormField>
          <FormField id="s-deposit" label="Sinal (% do total da reserva)" required error={errors.deposit_percent?.message}>
            <Input id="s-deposit" type="number" step="0.01" {...register("deposit_percent", number)} />
          </FormField>
          <FormField id="s-deposit-after" label="Sinal vence em até (dias após a reserva)" required error={errors.deposit_due_days_after_booking?.message}>
            <Input id="s-deposit-after" type="number" {...register("deposit_due_days_after_booking", number)} />
          </FormField>
          <FormField id="s-deposit-before" label="Sinal vence, no máximo, (dias antes da chegada)" required error={errors.deposit_due_days_before_arrival?.message}>
            <Input id="s-deposit-before" type="number" {...register("deposit_due_days_before_arrival", number)} />
          </FormField>
          <div className="grid gap-3 sm:col-span-2">
            {message && (
              <p role={message.kind === "ok" ? "status" : "alert"} className={message.kind === "ok" ? "text-sm" : "text-sm text-destructive"}>
                {message.text}
              </p>
            )}
            {message?.conflict && (
              <Button type="button" variant="outline" className="w-fit" onClick={reload}>
                Recarregar valores salvos
              </Button>
            )}
            <RequiredLegend />
            <Button type="submit" className="w-fit" disabled={isSubmitting || !data}>
              Salvar configurações
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  );
}
