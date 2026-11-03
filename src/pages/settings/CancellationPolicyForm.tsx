import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  getCancellationPolicy,
  updateCancellationPolicy,
  type ApiError,
  type CancellationTier,
} from "@/lib/api";
import { useSessionStore } from "@/stores/session";

interface Row {
  days: string;
  percent: string;
}

function toRows(tiers: CancellationTier[]): Row[] {
  return tiers.map((t) => ({ days: String(t.min_days_before), percent: String(t.refund_bp / 100) }));
}

/** Converte as linhas digitadas em faixas; devolve a mensagem do primeiro problema encontrado. */
function toTiers(rows: Row[]): CancellationTier[] | string {
  const tiers: CancellationTier[] = [];
  for (const row of rows) {
    if (row.days.trim() === "" || row.percent.trim() === "") return "Preencha todas as faixas.";
    const days = Number(row.days);
    const percent = Number(row.percent);
    if (!Number.isInteger(days) || days < 0) return "A antecedência deve ser um número inteiro de dias, a partir de 0.";
    if (!Number.isFinite(percent) || percent < 0 || percent > 100) return "A devolução vai de 0% a 100%.";
    tiers.push({ min_days_before: days, refund_bp: Math.round(percent * 100) });
  }
  if (new Set(tiers.map((t) => t.min_days_before)).size !== tiers.length) {
    return "Não pode haver duas faixas com a mesma antecedência.";
  }
  if (!tiers.some((t) => t.min_days_before === 0)) return "Inclua a faixa de 0 dia.";
  return tiers;
}

export default function CancellationPolicyForm({ onDirtyChange }: { onDirtyChange: (dirty: boolean) => void }) {
  const token = useSessionStore((s) => s.token) ?? "";
  const canManage = useSessionStore((s) => s.user?.role === "admin");
  const queryClient = useQueryClient();
  const { data } = useQuery({ queryKey: ["cancellation-policy"], queryFn: () => getCancellationPolicy(token) });
  const saved = useMemo(() => (data ? toRows(data.tiers) : []), [data]);
  const [rows, setRows] = useState<Row[]>([]);
  const [message, setMessage] = useState<{ kind: "ok" | "error"; text: string } | null>(null);

  useEffect(() => setRows(saved), [saved]);

  const dirty = JSON.stringify(rows) !== JSON.stringify(saved);
  useEffect(() => onDirtyChange(dirty), [dirty, onDirtyChange]);

  const update = (index: number, key: keyof Row, value: string) =>
    setRows((current) => current.map((row, i) => (i === index ? { ...row, [key]: value } : row)));

  const save = async () => {
    setMessage(null);
    const tiers = toTiers(rows);
    if (typeof tiers === "string") {
      setMessage({ kind: "error", text: tiers });
      return;
    }
    try {
      await updateCancellationPolicy(token, tiers, data?.version ?? 0);
      await queryClient.invalidateQueries({ queryKey: ["cancellation-policy"] });
      setMessage({ kind: "ok", text: "Política de cancelamento salva." });
    } catch (e) {
      setMessage({ kind: "error", text: (e as ApiError).message });
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Política de cancelamento</CardTitle>
        <p className="text-sm text-muted-foreground">
          Quanto do sinal já pago é devolvido, conforme a antecedência do cancelamento. A multa incide só sobre o sinal.
          Valores provisórios, aguardando aprovação da pousada. No-show retém todo o sinal.
        </p>
      </CardHeader>
      <CardContent className="grid gap-4">
        <div className="grid gap-2">
          {rows.map((row, i) => (
            <div key={i} className="flex flex-wrap items-center gap-2 text-sm">
              <span>Com</span>
              <Input
                aria-label={`Dias de antecedência, faixa ${i + 1}`}
                type="number"
                className="w-24"
                value={row.days}
                disabled={!canManage}
                onChange={(e) => update(i, "days", e.target.value)}
              />
              <span>dias ou mais, devolve</span>
              <Input
                aria-label={`Devolução do sinal (%), faixa ${i + 1}`}
                type="number"
                step="0.01"
                className="w-24"
                value={row.percent}
                disabled={!canManage}
                onChange={(e) => update(i, "percent", e.target.value)}
              />
              <span>% do sinal</span>
              {canManage && (
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  aria-label={`Remover faixa ${i + 1}`}
                  onClick={() => setRows((current) => current.filter((_, index) => index !== i))}
                >
                  Remover
                </Button>
              )}
            </div>
          ))}
        </div>
        {message && (
          <p role={message.kind === "ok" ? "status" : "alert"} className={message.kind === "ok" ? "text-sm" : "text-sm text-destructive"}>
            {message.text}
          </p>
        )}
        {canManage && (
          <div className="flex gap-2">
            <Button type="button" variant="outline" onClick={() => setRows((current) => [...current, { days: "", percent: "" }])}>
              Adicionar faixa
            </Button>
            <Button type="button" onClick={save} disabled={!data}>
              Salvar política
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
