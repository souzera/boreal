import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { FormField } from "@/components/FormField";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import {
  createRateRule,
  listCategories,
  listRateRules,
  quoteStay,
  setRateRuleActive,
  updateRateRule,
  type ApiError,
  type AppliedRule,
  type Quote,
  type RateRule,
  type RateRuleInput,
  type RateRuleType,
} from "@/lib/api";
import { formatDate, formatMoney } from "@/lib/format";
import { useSessionStore } from "@/stores/session";
import HolidaysPanel from "./HolidaysPanel";
import RuleForm, { weekdayNames } from "./RuleForm";

const appliedLabel: Record<AppliedRule, string> = {
  date_override: "Exceção de data",
  holiday: "Feriado",
  season: "Temporada",
  weekday: "Dia da semana",
};

interface FormState {
  open: boolean;
  type: RateRuleType;
  rule: RateRule | null;
  weekday?: number;
}

const closed: FormState = { open: false, type: "weekday", rule: null };

export default function RatesPage() {
  const token = useSessionStore((s) => s.token) ?? "";
  const canManage = useSessionStore((s) => s.user?.role === "admin");
  const queryClient = useQueryClient();
  const { data: categories = [] } = useQuery({ queryKey: ["categories", "active"], queryFn: () => listCategories(token, true) });
  const [chosen, setChosen] = useState<number | null>(null);
  const categoryId = chosen ?? categories[0]?.id ?? null;
  const { data: rules = [] } = useQuery({
    queryKey: ["rate-rules", categoryId],
    queryFn: () => listRateRules(token, categoryId as number),
    enabled: categoryId !== null,
  });
  const [form, setForm] = useState<FormState>(closed);
  const [notice, setNotice] = useState<{ kind: "ok" | "error"; text: string } | null>(null);
  const [entry, setEntry] = useState("");
  const [exit, setExit] = useState("");
  const [quote, setQuote] = useState<Quote | null>(null);
  const [quoteError, setQuoteError] = useState<string | null>(null);

  const refresh = () => queryClient.invalidateQueries({ queryKey: ["rate-rules"] });
  const byType = (type: RateRuleType) => rules.filter((r) => r.rule_type === type);
  const weekdayRule = (day: number) => byType("weekday").find((r) => r.weekday === day && r.active);

  const save = async (input: RateRuleInput) => {
    if (form.rule) await updateRateRule(token, form.rule.id, input, form.rule.version);
    else await createRateRule(token, input);
    await refresh();
    setForm(closed);
    setNotice({ kind: "ok", text: "Tarifa salva." });
  };

  const toggle = async (rule: RateRule) => {
    setNotice(null);
    try {
      await setRateRuleActive(token, rule.id, !rule.active);
      await refresh();
      setNotice({ kind: "ok", text: rule.active ? "Tarifa desativada." : "Tarifa reativada." });
    } catch (e) {
      setNotice({ kind: "error", text: (e as ApiError).message });
    }
  };

  const simulate = async () => {
    setQuoteError(null);
    setQuote(null);
    if (categoryId === null || !entry || !exit) {
      setQuoteError("Informe a entrada e a saída.");
      return;
    }
    try {
      setQuote(await quoteStay(token, categoryId, entry, exit));
    } catch (e) {
      setQuoteError((e as ApiError).message);
    }
  };

  const editButtons = (rule: RateRule, type: RateRuleType) =>
    canManage && (
      <div className="flex gap-1">
        <Button size="sm" variant="outline" onClick={() => setForm({ open: true, type, rule })}>
          Editar
        </Button>
        <Button size="sm" variant="outline" onClick={() => toggle(rule)}>
          {rule.active ? "Desativar" : "Reativar"}
        </Button>
      </div>
    );

  const holidayRule = byType("holiday")[0];

  return (
    <section className="grid max-w-5xl gap-8">
      <PageHeader title="Tarifas" description="Preço da diária por categoria de quarto. A ordem de prioridade é: exceção de data, feriado, temporada e dia da semana." />
      <div className="w-64">
        <FormField id="rates-category" label="Categoria">
          <select
            id="rates-category"
            className="h-8 rounded-lg border border-input bg-transparent px-2 text-sm"
            value={categoryId ?? ""}
            onChange={(e) => setChosen(Number(e.target.value))}
          >
            {categories.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </select>
        </FormField>
      </div>
      {notice && (
        <p role={notice.kind === "ok" ? "status" : "alert"} className={notice.kind === "ok" ? "text-sm" : "text-sm text-destructive"}>
          {notice.text}
        </p>
      )}

      <section className="grid gap-3">
        <h2 className="text-lg font-semibold">Dia da semana</h2>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Dia</TableHead>
              <TableHead>Diária</TableHead>
              {canManage && <TableHead>Ações</TableHead>}
            </TableRow>
          </TableHeader>
          <TableBody>
            {weekdayNames.map((name, day) => {
              const rule = weekdayRule(day);
              return (
                <TableRow key={day}>
                  <TableCell>{name}</TableCell>
                  <TableCell>{rule ? formatMoney(rule.amount_cents) : "Sem tarifa"}</TableCell>
                  {canManage && (
                    <TableCell>
                      {rule ? (
                        editButtons(rule, "weekday")
                      ) : (
                        <Button size="sm" variant="outline" onClick={() => setForm({ open: true, type: "weekday", rule: null, weekday: day })}>
                          Definir
                        </Button>
                      )}
                    </TableCell>
                  )}
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      </section>

      <section className="grid gap-3">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold">Temporadas</h2>
          {canManage && <Button onClick={() => setForm({ open: true, type: "season", rule: null })}>Nova temporada</Button>}
        </div>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Descrição</TableHead>
              <TableHead>Período</TableHead>
              <TableHead>Diária</TableHead>
              <TableHead>Situação</TableHead>
              {canManage && <TableHead>Ações</TableHead>}
            </TableRow>
          </TableHeader>
          <TableBody>
            {byType("season").length === 0 ? (
              <TableRow>
                <TableCell colSpan={5} className="text-center text-muted-foreground">
                  Nenhuma temporada cadastrada.
                </TableCell>
              </TableRow>
            ) : (
              byType("season").map((r) => (
                <TableRow key={r.id}>
                  <TableCell>{r.label || "Temporada"}</TableCell>
                  <TableCell>{`${formatDate(r.start_date ?? "")} a ${formatDate(r.end_date ?? "")}`}</TableCell>
                  <TableCell>{formatMoney(r.amount_cents)}</TableCell>
                  <TableCell>{r.active ? "Ativa" : "Inativa"}</TableCell>
                  {canManage && <TableCell>{editButtons(r, "season")}</TableCell>}
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </section>

      <section className="grid gap-3">
        <h2 className="text-lg font-semibold">Feriados e exceções de data</h2>
        <div className="flex flex-wrap items-center gap-3 text-sm">
          <span>Preço nos feriados cadastrados</span>
          <strong>{holidayRule ? formatMoney(holidayRule.amount_cents) : "Sem tarifa"}</strong>
          {canManage &&
            (holidayRule ? (
              editButtons(holidayRule, "holiday")
            ) : (
              <Button size="sm" variant="outline" onClick={() => setForm({ open: true, type: "holiday", rule: null })}>
                Definir
              </Button>
            ))}
        </div>
        <div className="flex items-center justify-between">
          <h3 className="font-medium">Exceções de data</h3>
          {canManage && (
            <Button variant="outline" onClick={() => setForm({ open: true, type: "date_override", rule: null })}>
              Nova exceção de data
            </Button>
          )}
        </div>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Descrição</TableHead>
              <TableHead>Data</TableHead>
              <TableHead>Diária</TableHead>
              <TableHead>Situação</TableHead>
              {canManage && <TableHead>Ações</TableHead>}
            </TableRow>
          </TableHeader>
          <TableBody>
            {byType("date_override").length === 0 ? (
              <TableRow>
                <TableCell colSpan={5} className="text-center text-muted-foreground">
                  Nenhuma exceção de data.
                </TableCell>
              </TableRow>
            ) : (
              byType("date_override").map((r) => (
                <TableRow key={r.id}>
                  <TableCell>{r.label || "Exceção"}</TableCell>
                  <TableCell>{formatDate(r.date ?? "")}</TableCell>
                  <TableCell>{formatMoney(r.amount_cents)}</TableCell>
                  <TableCell>{r.active ? "Ativa" : "Inativa"}</TableCell>
                  {canManage && <TableCell>{editButtons(r, "date_override")}</TableCell>}
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </section>

      <HolidaysPanel canManage={!!canManage} />

      <section className="grid gap-3">
        <h2 className="text-lg font-semibold">Simular preço</h2>
        <div className="flex flex-wrap items-end gap-3">
          <FormField id="sim-in" label="Entrada">
            <Input id="sim-in" type="date" value={entry} onChange={(e) => setEntry(e.target.value)} />
          </FormField>
          <FormField id="sim-out" label="Saída">
            <Input id="sim-out" type="date" value={exit} onChange={(e) => setExit(e.target.value)} />
          </FormField>
          <Button onClick={simulate}>Simular preço</Button>
        </div>
        {quoteError && (
          <p role="alert" className="text-sm text-destructive">
            {quoteError}
          </p>
        )}
        {quote && (
          <div className="grid gap-2">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Noite</TableHead>
                  <TableHead>Diária</TableHead>
                  <TableHead>Regra aplicada</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {quote.nights.map((n) => (
                  <TableRow key={n.date}>
                    <TableCell>{formatDate(n.date)}</TableCell>
                    <TableCell>{formatMoney(n.amount_cents)}</TableCell>
                    <TableCell>{appliedLabel[n.rule]}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
            <p className="text-sm">
              Total: <strong>{formatMoney(quote.total_cents)}</strong>
            </p>
            {quote.missing_dates.length > 0 && (
              <p role="alert" className="text-sm text-destructive">
                {`Faltam tarifas para: ${quote.missing_dates.map(formatDate).join(", ")}. Não é possível reservar essas datas.`}
              </p>
            )}
          </div>
        )}
      </section>

      {categoryId !== null && (
        <RuleForm
          open={form.open}
          categoryId={categoryId}
          type={form.type}
          rule={form.rule}
          weekday={form.weekday}
          onClose={() => setForm(closed)}
          onSubmit={save}
        />
      )}
    </section>
  );
}
