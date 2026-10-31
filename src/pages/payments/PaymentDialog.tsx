import { useEffect, useState } from "react";

import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { useGuardedClose } from "@/hooks/useGuardedClose";
import { usePermissions } from "@/hooks/usePermissions";
import {
  recordReservationPayment,
  recordStayPayment,
  type ApiError,
  type DuplicateInfo,
  type PaymentInput,
} from "@/lib/api";
import { formatMoney, parseMoney } from "@/lib/format";
import { checkSplit, describeComponents, emptyRow, type SplitRow } from "@/lib/payment";
import { useSessionStore } from "@/stores/session";
import ComponentsEditor from "./ComponentsEditor";
import DuplicateDialog from "./DuplicateDialog";

type Target = { kind: "stay"; stayId: number } | { kind: "reservation"; reservationId: number; version: number };

interface Props {
  open: boolean;
  target: Target;
  /** Situação da conta ou do sinal, em palavras, mostrada abaixo do título. */
  hint: string;
  onClose: () => void;
  onDone: (message: string) => void;
  onConflict?: () => void;
}

type Step = "form" | "confirm" | "duplicate";

/** Registro de um recebimento (conta da hospedagem ou sinal da reserva), dividido entre formas de pagamento. */
export default function PaymentDialog({ open, target, hint, onClose, onDone, onConflict }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const { can } = usePermissions();
  const [total, setTotal] = useState("");
  const [rows, setRows] = useState<SplitRow[]>([emptyRow()]);
  const [note, setNote] = useState("");
  const [step, setStep] = useState<Step>("form");
  const [errors, setErrors] = useState<{ total?: string; split?: string }>({});
  const [message, setMessage] = useState<string | null>(null);
  const [duplicate, setDuplicate] = useState<DuplicateInfo | null>(null);
  const [justification, setJustification] = useState("");
  const [releaseError, setReleaseError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    setTotal("");
    setRows([emptyRow()]);
    setNote("");
    setStep("form");
    setErrors({});
    setMessage(null);
    setDuplicate(null);
    setJustification("");
    setReleaseError(null);
  }, [open]);

  const { requestClose, confirm } = useGuardedClose(total !== "" || note !== "", onClose);

  const totalCents = parseMoney(total);
  const check = checkSplit(totalCents, rows);
  const splitMismatch = rows.length > 1 && totalCents !== null && check.remaining !== 0;

  const review = () => {
    setMessage(null);
    if (check.error) {
      const next: typeof errors = {};
      if (totalCents === null || totalCents <= 0) next.total = check.error;
      else next.split = check.error;
      setErrors(next);
      return;
    }
    setErrors({});
    setStep("confirm");
  };

  const send = async (release?: string) => {
    if (!check.components) return;
    const input: PaymentInput = { components: check.components, note: note.trim() };
    if (release !== undefined) input.duplicate_release = { justification: release };
    setBusy(true);
    try {
      const receipt =
        target.kind === "stay"
          ? await recordStayPayment(token, target.stayId, input)
          : await recordReservationPayment(token, target.reservationId, target.version, input);
      onDone(`Pagamento ${receipt.code} de ${formatMoney(receipt.total_cents)} registrado e salvo neste computador.`);
    } catch (e) {
      const apiError = e as ApiError;
      if (apiError.code === "duplicate_payment" && apiError.details) {
        setDuplicate(apiError.details as DuplicateInfo);
        setStep("duplicate");
        return;
      }
      if (release !== undefined) {
        // a liberação falhou: continua na tela do duplicado, com os dados e a justificativa
        setReleaseError(apiError.message);
        return;
      }
      setStep("form");
      setMessage(apiError.message);
      if (apiError.code === "conflict") onConflict?.();
    } finally {
      setBusy(false);
    }
  };

  const release = () => {
    const why = justification.trim();
    if (!why) {
      setReleaseError("Informe a justificativa para liberar o recebimento.");
      return;
    }
    setReleaseError(null);
    void send(why);
  };

  const summary =
    check.components && check.components.length === 1
      ? `${formatMoney(check.sum)} em ${describeComponents(check.components).replace(/ R\$ .*$/, "")}`
      : `${formatMoney(check.sum)} dividido em ${check.components ? describeComponents(check.components) : ""}`;

  return (
    <>
      <Dialog open={open} onOpenChange={(next) => !next && requestClose()}>
        <DialogContent showCloseButton={false}>
          <DialogHeader>
            <DialogTitle>Registrar pagamento</DialogTitle>
            <DialogDescription>{hint}</DialogDescription>
          </DialogHeader>
          {step === "form" && (
            <div className="grid gap-3">
              <FormField id="pay-total" label="Valor recebido" required error={errors.total}>
                <Input id="pay-total" inputMode="decimal" placeholder="R$ 0,00" value={total} onChange={(e) => setTotal(e.target.value)} />
              </FormField>
              <ComponentsEditor totalCents={totalCents} rows={rows} onChange={setRows} error={errors.split} />
              <FormField id="pay-note" label="Observação">
                <Input id="pay-note" value={note} onChange={(e) => setNote(e.target.value)} />
              </FormField>
            </div>
          )}
          {step === "confirm" && (
            <p className="text-sm font-medium">
              {`Registrar pagamento de ${summary}? Depois de salvo, o valor não pode ser corrigido nesta tela.`}
            </p>
          )}
          {step === "duplicate" && duplicate && (
            <DuplicateDialog
              info={duplicate}
              canRelease={can("release_duplicate_payment")}
              justification={justification}
              error={releaseError}
              busy={busy}
              onJustification={setJustification}
              onRelease={release}
              onBack={() => setStep("form")}
            />
          )}
          {message && (
            <p role="alert" className="text-sm text-destructive">
              {message}
            </p>
          )}
          {step !== "duplicate" && (
            <DialogFooter>
              {step === "form" ? (
                <>
                  <Button type="button" variant="outline" onClick={requestClose}>
                    Cancelar
                  </Button>
                  <Button type="button" disabled={splitMismatch} onClick={review}>
                    Registrar pagamento
                  </Button>
                </>
              ) : (
                <>
                  <Button type="button" variant="outline" onClick={() => setStep("form")}>
                    Corrigir dados
                  </Button>
                  <Button type="button" disabled={busy} onClick={() => void send()}>
                    Confirmar pagamento
                  </Button>
                </>
              )}
            </DialogFooter>
          )}
        </DialogContent>
      </Dialog>
      {confirm}
    </>
  );
}
