import { zodResolver } from "@hookform/resolvers/zod";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { Link, useNavigate, useParams } from "react-router-dom";
import { z } from "zod";

import { ConfirmDialog } from "@/components/ConfirmDialog";
import { FormField, RequiredLegend } from "@/components/FormField";
import { PageHeader } from "@/components/PageHeader";
import { UnsavedChangesGuard } from "@/components/UnsavedChangesGuard";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  createGuest,
  getGuest,
  setGuestActive,
  updateGuest,
  type ApiError,
  type DuplicateMatch,
  type Guest,
  type GuestInput,
} from "@/lib/api";
import { isValidCpf } from "@/lib/cpf";
import { formatCpf, formatPhone, onlyDigits } from "@/lib/format";
import { useSessionStore } from "@/stores/session";
import DuplicatesDialog from "./DuplicatesDialog";

const schema = z.object({
  full_name: z.string().trim().min(1, "Informe o nome completo.").max(120, "Nome: no máximo 120 caracteres."),
  phone: z
    .string()
    .trim()
    .min(1, "Informe o telefone.")
    .refine((v) => [10, 11, 12, 13].includes(onlyDigits(v).length), "Telefone inválido. Informe o DDD e o número."),
  cpf: z.string().trim().refine((v) => v === "" || isValidCpf(v), "CPF inválido. Confira os números."),
  email: z
    .string()
    .trim()
    .refine((v) => v === "" || (v.includes("@") && !v.includes(" ")), "E-mail inválido."),
  address: z.string().trim().max(255, "Endereço: no máximo 255 caracteres."),
});
type Values = z.infer<typeof schema>;

const emptyValues: Values = { full_name: "", phone: "", cpf: "", email: "", address: "" };

function toValues(g: Guest): Values {
  return {
    full_name: g.full_name,
    phone: formatPhone(g.phone),
    cpf: g.cpf ? formatCpf(g.cpf) : "",
    email: g.email,
    address: g.address,
  };
}

function toInput(v: Values): GuestInput {
  return { ...v, cpf: v.cpf === "" ? null : v.cpf };
}

export default function GuestFormPage() {
  const { id } = useParams();
  const guestId = id && id !== "novo" ? Number(id) : null;
  const token = useSessionStore((s) => s.token) ?? "";
  const isAdmin = useSessionStore((s) => s.user?.role === "admin");
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { data: guest } = useQuery({
    queryKey: ["guest", guestId],
    queryFn: () => getGuest(token, guestId as number),
    enabled: guestId !== null,
  });
  const [duplicates, setDuplicates] = useState<DuplicateMatch[] | null>(null);
  const [pendingInput, setPendingInput] = useState<GuestInput | null>(null);
  const [formError, setFormError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [confirmDeactivate, setConfirmDeactivate] = useState(false);
  // Sair só depois de a tela ser renderizada sem o aviso de alterações não salvas.
  const [leaveTo, setLeaveTo] = useState<string | null>(null);
  useEffect(() => {
    if (leaveTo) navigate(leaveTo);
  }, [leaveTo, navigate]);
  const {
    register,
    handleSubmit,
    reset,
    formState: { errors, isDirty, isSubmitting },
  } = useForm<Values>({
    resolver: zodResolver(schema),
    values: guest ? toValues(guest) : emptyValues,
  });

  const masked = (mask: (v: string) => string) => ({
    onChange: (e: { target: { value: string } }) => {
      e.target.value = mask(e.target.value);
    },
  });

  const finishCreate = async () => {
    await queryClient.invalidateQueries({ queryKey: ["guests"] });
    setLeaveTo("/hospedes");
  };

  const submitCreate = async (input: GuestInput, confirm: boolean) => {
    const outcome = await createGuest(token, input, confirm);
    if (outcome.status === "duplicates") {
      setPendingInput(input);
      setDuplicates(outcome.matches);
      return;
    }
    setDuplicates(null);
    await finishCreate();
  };

  const onSubmit = handleSubmit(async (values) => {
    setFormError(null);
    setNotice(null);
    const input = toInput(values);
    try {
      if (guest) {
        await updateGuest(token, guest.id, input, guest.version);
        await queryClient.invalidateQueries({ queryKey: ["guest", guest.id] });
        await queryClient.invalidateQueries({ queryKey: ["guests"] });
        reset(values);
        setNotice("Dados do hóspede salvos.");
      } else {
        await submitCreate(input, false);
      }
    } catch (e) {
      setFormError((e as ApiError).message);
    }
  });

  const createAnother = async () => {
    if (!pendingInput) return;
    try {
      await submitCreate(pendingInput, true);
    } catch (e) {
      setDuplicates(null);
      setFormError((e as ApiError).message);
    }
  };

  const deactivate = async () => {
    setConfirmDeactivate(false);
    if (!guest) return;
    try {
      await setGuestActive(token, guest.id, !guest.active);
      await queryClient.invalidateQueries({ queryKey: ["guest", guest.id] });
      await queryClient.invalidateQueries({ queryKey: ["guests"] });
      setNotice(guest.active ? "Hóspede desativado." : "Hóspede reativado.");
    } catch (e) {
      setFormError((e as ApiError).message);
    }
  };

  return (
    <section className="max-w-2xl">
      <PageHeader
        title={guest ? guest.full_name : "Novo hóspede"}
        description={guest ? `${guest.code}${guest.active ? "" : " · cadastro inativo"}` : "Preencha os dados do responsável pela hospedagem."}
        action={
          <Button variant="outline" render={<Link to="/hospedes" />}>
            Voltar à lista
          </Button>
        }
      />
      <form onSubmit={onSubmit} noValidate className="grid gap-4 sm:grid-cols-2">
        <div className="sm:col-span-2">
          <FormField id="g-name" label="Nome completo" required error={errors.full_name?.message}>
            <Input id="g-name" autoComplete="off" {...register("full_name")} />
          </FormField>
        </div>
        <FormField id="g-phone" label="Telefone" required error={errors.phone?.message}>
          <Input id="g-phone" inputMode="tel" {...register("phone", masked(formatPhone))} />
        </FormField>
        <FormField id="g-cpf" label="CPF" error={errors.cpf?.message}>
          <Input id="g-cpf" inputMode="numeric" {...register("cpf", masked(formatCpf))} />
        </FormField>
        <FormField id="g-email" label="E-mail" error={errors.email?.message}>
          <Input id="g-email" type="email" {...register("email")} />
        </FormField>
        <FormField id="g-address" label="Endereço" error={errors.address?.message}>
          <Input id="g-address" {...register("address")} />
        </FormField>
        <div className="grid gap-3 sm:col-span-2">
          {notice && <p role="status" className="text-sm">{notice}</p>}
          {formError && (
            <p role="alert" className="text-sm text-destructive">
              {formError}
            </p>
          )}
          <RequiredLegend />
          <div className="flex flex-wrap gap-2">
            <Button type="submit" disabled={isSubmitting}>
              Salvar hóspede
            </Button>
            {guest && isAdmin && (
              <Button type="button" variant="outline" onClick={() => setConfirmDeactivate(true)}>
                {guest.active ? "Desativar hóspede" : "Reativar hóspede"}
              </Button>
            )}
          </div>
        </div>
      </form>

      <UnsavedChangesGuard isDirty={isDirty && leaveTo === null} />
      <DuplicatesDialog
        matches={duplicates}
        busy={isSubmitting}
        onCancel={() => setDuplicates(null)}
        onCreateAnother={createAnother}
        onUseExisting={(g) => {
          setDuplicates(null);
          setLeaveTo(`/hospedes/${g.id}`);
        }}
      />
      <ConfirmDialog
        open={confirmDeactivate}
        title={guest?.active ? "Desativar hóspede?" : "Reativar hóspede?"}
        description={
          guest?.active
            ? "O cadastro deixa de aparecer nas buscas para novas reservas. O histórico é mantido."
            : "O cadastro volta a aparecer nas buscas."
        }
        confirmLabel="Confirmar desativação"
        destructive={guest?.active}
        onCancel={() => setConfirmDeactivate(false)}
        onConfirm={deactivate}
      />
    </section>
  );
}
