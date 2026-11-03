import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { ConfirmDialog } from "@/components/ConfirmDialog";
import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { createHoliday, deleteHoliday, listHolidays, type ApiError, type Holiday } from "@/lib/api";
import { formatDate } from "@/lib/format";
import { useSessionStore } from "@/stores/session";

/** Calendário de feriados: o preço de feriado de cada categoria vale nestas datas. */
export default function HolidaysPanel({ canManage }: { canManage: boolean }) {
  const token = useSessionStore((s) => s.token) ?? "";
  const queryClient = useQueryClient();
  const { data = [] } = useQuery({ queryKey: ["holidays"], queryFn: () => listHolidays(token) });
  const [date, setDate] = useState("");
  const [name, setName] = useState("");
  const [toRemove, setToRemove] = useState<Holiday | null>(null);
  const [error, setError] = useState<string | null>(null);
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["holidays"] });

  const add = async () => {
    setError(null);
    if (!date || !name.trim()) {
      setError("Informe a data e o nome do feriado.");
      return;
    }
    try {
      await createHoliday(token, date, name.trim());
      setDate("");
      setName("");
      await refresh();
    } catch (e) {
      setError((e as ApiError).message);
    }
  };

  const remove = async () => {
    const holiday = toRemove;
    setToRemove(null);
    if (!holiday) return;
    try {
      await deleteHoliday(token, holiday.id);
      await refresh();
    } catch (e) {
      setError((e as ApiError).message);
    }
  };

  return (
    <section className="grid gap-3">
      <h2 className="text-lg font-semibold">Calendário de feriados</h2>
      <p className="text-sm text-muted-foreground">
        Cadastre os feriados de cada ano, inclusive os municipais. O preço de feriado de cada categoria vale nessas datas.
      </p>
      {canManage && (
        <div className="flex flex-wrap items-end gap-3">
          <FormField id="holiday-date" label="Data do feriado">
            <Input id="holiday-date" type="date" value={date} onChange={(e) => setDate(e.target.value)} />
          </FormField>
          <FormField id="holiday-name" label="Nome do feriado">
            <Input id="holiday-name" value={name} onChange={(e) => setName(e.target.value)} />
          </FormField>
          <Button onClick={add}>Adicionar feriado</Button>
        </div>
      )}
      {error && (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      )}
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Data</TableHead>
            <TableHead>Feriado</TableHead>
            {canManage && <TableHead>Ações</TableHead>}
          </TableRow>
        </TableHeader>
        <TableBody>
          {data.length === 0 ? (
            <TableRow>
              <TableCell colSpan={3} className="text-center text-muted-foreground">
                Nenhum feriado cadastrado.
              </TableCell>
            </TableRow>
          ) : (
            data.map((h) => (
              <TableRow key={h.id}>
                <TableCell>{formatDate(h.date)}</TableCell>
                <TableCell>{h.name}</TableCell>
                {canManage && (
                  <TableCell>
                    <Button size="sm" variant="outline" onClick={() => setToRemove(h)}>
                      Remover
                    </Button>
                  </TableCell>
                )}
              </TableRow>
            ))
          )}
        </TableBody>
      </Table>
      <ConfirmDialog
        open={toRemove !== null}
        title="Remover feriado?"
        description={`${toRemove?.name} deixa de usar o preço de feriado.`}
        confirmLabel="Remover feriado"
        destructive
        onCancel={() => setToRemove(null)}
        onConfirm={remove}
      />
    </section>
  );
}
