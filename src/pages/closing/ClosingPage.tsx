import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";

import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { closeCash, closingPendingDates, closingPreview, type ApiError, type ExpectedLineView } from "@/lib/api";
import { formatDate, formatInstant, formatMoney, parseMoney, toLocalIsoDate } from "@/lib/format";
import { useSessionStore } from "@/stores/session";
import ClosingDialog from "./ClosingDialog";
import Icon from "@/components/Icon";

/** Valor conferido em reais; aceita sinal de menos porque o esperado do dia pode ser negativo (mais devoluções que recebimentos). */
function parseCounted(text: string): number | null {
  const trimmed = text.trim();
  if (trimmed.startsWith("-")) {
    const rest = parseMoney(trimmed.slice(1));
    return rest === null ? null : -rest;
  }
  return parseMoney(trimmed);
}

/** Situação da diferença em palavras e ícone: nunca só cor. */
function Difference({ cents }: { cents: number }) {
  if (cents === 0) {
    return (
      <>
        <Icon name="check_circle" size="xs" className="mr-1 align-middle" />
        Sem diferença
      </>
    );
  }
  return (
    <>
      <Icon name="warning" size="xs" className="mr-1 align-middle" />
      {cents < 0 ? `Falta ${formatMoney(-cents)}` : `Sobra ${formatMoney(cents)}`}
    </>
  );
}

export default function ClosingPage() {
  const { data: dateParam } = useParams();
  const navigate = useNavigate();
  const token = useSessionStore((s) => s.token) ?? "";
  const queryClient = useQueryClient();
  const today = toLocalIsoDate(new Date());
  const date = dateParam ?? today;
  const [counts, setCounts] = useState<Record<string, string>>({});
  const [reasons, setReasons] = useState<Record<string, string>>({});
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [message, setMessage] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);

  const preview = useQuery({ queryKey: ["closing", date], queryFn: () => closingPreview(token, date) });
  const pending = useQuery({ queryKey: ["closing-pending"], queryFn: () => closingPendingDates(token) });

  useEffect(() => {
    setCounts({});
    setReasons({});
    setErrors({});
    setMessage(null);
    setConfirming(false);
  }, [date]);

  const data = preview.data;
  const lines: ExpectedLineView[] = data?.expected ?? [];
  const countedCents = (line: ExpectedLineView) => parseCounted(counts[line.method] ?? "");
  const differenceOf = (line: ExpectedLineView) => {
    const counted = countedCents(line);
    return counted === null ? null : counted - line.expected_cents;
  };

  const review = () => {
    setMessage(null);
    const next: Record<string, string> = {};
    for (const line of lines) {
      const diff = differenceOf(line);
      if (diff === null) next[line.method] = "Informe o valor conferido, por exemplo R$ 0,00.";
      else if (diff !== 0 && (reasons[line.method] ?? "").trim() === "") next[line.method] = `Explique a diferença de ${line.label}.`;
    }
    setErrors(next);
    if (Object.keys(next).length === 0) setConfirming(true);
  };

  const conclude = async () => {
    setConfirming(false);
    setBusy(true);
    try {
      await closeCash(token, {
        date,
        counted: lines.map((line) => ({
          method: line.method,
          counted_cents: countedCents(line) ?? 0,
          justification: differenceOf(line) === 0 ? null : (reasons[line.method] ?? "").trim() || null,
        })),
      });
      setNotice(`Fechamento de ${formatDate(date)} concluído e salvo neste computador.`);
      await queryClient.invalidateQueries({ queryKey: ["closing"] });
      await queryClient.invalidateQueries({ queryKey: ["closing-pending"] });
    } catch (e) {
      setMessage((e as ApiError).message);
      // o esperado pode ter mudado (novo movimento): recarrega a conferência sem apagar o que foi digitado
      await queryClient.invalidateQueries({ queryKey: ["closing"] });
    } finally {
      setBusy(false);
    }
  };

  const differences = lines
    .map((line) => ({ label: line.label, cents: differenceOf(line) ?? 0 }))
    .filter((d) => d.cents !== 0);

  return (
    <section className="grid gap-6">
      <PageHeader title="Fechamento de caixa" description="Confira, por forma de pagamento, o que entrou e o que foi devolvido no dia." />
      {notice && (
        <p role="status" className="text-sm">
          {notice}
        </p>
      )}

      <div className="flex flex-wrap items-end gap-4">
        <label className="grid gap-1 text-sm">
          Dia a fechar
          <Input
            type="date"
            aria-label="Dia a fechar"
            max={today}
            value={date}
            onChange={(e) => e.target.value && navigate(`/fechamento/${e.target.value}`)}
          />
        </label>
        {(pending.data ?? []).length > 0 && (
          <p className="text-sm">
            <span className="font-medium">
              <Icon name="warning" size="xs" className="mr-1 align-middle" />
              Dias sem fechamento:{" "}
            </span>
            {(pending.data ?? []).map((d, i) => (
              <span key={d}>
                {i > 0 && ", "}
                <Link to={`/fechamento/${d}`} className="underline">
                  {formatDate(d)}
                </Link>
              </span>
            ))}
          </p>
        )}
      </div>

      {data?.closed ? (
        <section aria-label="Fechamento concluído" className="grid gap-3">
          <p className="text-sm font-medium">
            {`Fechado por ${data.closed.closed_by_name} em ${formatInstant(data.closed.closed_at)}.`}
          </p>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Forma de pagamento</TableHead>
                <TableHead>Esperado</TableHead>
                <TableHead>Conferido</TableHead>
                <TableHead>Diferença</TableHead>
                <TableHead>Justificativa</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.closed.lines.map((l) => (
                <TableRow key={l.method}>
                  <TableCell>{l.label}</TableCell>
                  <TableCell>{formatMoney(l.expected_cents)}</TableCell>
                  <TableCell>{formatMoney(l.counted_cents)}</TableCell>
                  <TableCell>
                    <Difference cents={l.difference_cents} />
                  </TableCell>
                  <TableCell>{l.justification}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
          {data.after_closing_cents !== null && data.after_closing_cents !== 0 && (
            <p role="status" className="text-sm font-medium">
              {`Movimentos depois do fechamento: ${formatMoney(data.after_closing_cents)}`}
            </p>
          )}
        </section>
      ) : (
        data && (
          <section aria-label="Conferência do dia" className="grid gap-3">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Forma de pagamento</TableHead>
                  <TableHead>Recebido</TableHead>
                  <TableHead>Devolvido</TableHead>
                  <TableHead>Esperado</TableHead>
                  <TableHead>Conferido</TableHead>
                  <TableHead>Diferença</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {lines.map((line) => {
                  const diff = differenceOf(line);
                  return (
                    <TableRow key={line.method}>
                      <TableCell>{line.label}</TableCell>
                      <TableCell>{formatMoney(line.received_cents)}</TableCell>
                      <TableCell>{formatMoney(line.refunded_cents)}</TableCell>
                      <TableCell>{formatMoney(line.expected_cents)}</TableCell>
                      <TableCell>
                        <Input
                          aria-label={`Conferido em ${line.label}`}
                          inputMode="decimal"
                          placeholder="R$ 0,00"
                          value={counts[line.method] ?? ""}
                          onChange={(e) => setCounts({ ...counts, [line.method]: e.target.value })}
                        />
                      </TableCell>
                      <TableCell>
                        {diff !== null && (
                          <p className="text-sm">
                            <Difference cents={diff} />
                          </p>
                        )}
                        {diff !== null && diff !== 0 && (
                          <Input
                            aria-label={`Justificativa de ${line.label}`}
                            placeholder="Por que há diferença?"
                            value={reasons[line.method] ?? ""}
                            onChange={(e) => setReasons({ ...reasons, [line.method]: e.target.value })}
                          />
                        )}
                        {errors[line.method] && (
                          <p role="alert" className="text-sm text-destructive">
                            {errors[line.method]}
                          </p>
                        )}
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
            <p className="text-sm text-muted-foreground">
              {`Pagamentos a fornecedores no dia: ${formatMoney(data.supplier_payments_cents)} (não entram no valor esperado).`}
            </p>
            {message && (
              <p role="alert" className="text-sm text-destructive">
                {message}
              </p>
            )}
            <div>
              <Button disabled={busy} onClick={review}>
                Concluir fechamento
              </Button>
            </div>
          </section>
        )
      )}

      <ClosingDialog open={confirming} date={date} differences={differences} onConfirm={() => void conclude()} onCancel={() => setConfirming(false)} />
    </section>
  );
}
