import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Link } from "react-router-dom";

import { FormField } from "@/components/FormField";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { listStays, type StayStatus } from "@/lib/api";
import { formatDate, formatMoney } from "@/lib/format";
import { stayStatusIcon, stayStatusLabel } from "@/lib/stay";
import { useSessionStore } from "@/stores/session";

const PAGE_SIZE = 50;

export default function StaysPage() {
  const token = useSessionStore((s) => s.token) ?? "";
  const [status, setStatus] = useState<StayStatus>("in_house");
  const [text, setText] = useState("");
  const query = { status, text: text || undefined, page: 1, page_size: PAGE_SIZE };
  const { data } = useQuery({ queryKey: ["stays", query], queryFn: () => listStays(token, query) });

  return (
    <section>
      <PageHeader
        title="Hospedagens"
        description="Quem está hospedado agora, com a conta e os avisos de saída."
        action={<Button render={<Link to="/hospedagens/nova" />}>Hospedagem sem reserva</Button>}
      />
      <div className="mb-4 flex flex-wrap items-end gap-4">
        <div className="w-56">
          <FormField id="stay-search" label="Buscar hospedagem">
            <Input id="stay-search" placeholder="Código, hóspede ou quarto" value={text} onChange={(e) => setText(e.target.value)} />
          </FormField>
        </div>
        <FormField id="stay-status" label="Situação">
          <select
            id="stay-status"
            className="h-8 rounded-lg border border-input bg-transparent px-2 text-sm"
            value={status}
            onChange={(e) => setStatus(e.target.value as StayStatus)}
          >
            <option value="in_house">{stayStatusLabel.in_house}</option>
            <option value="checked_out">{stayStatusLabel.checked_out}</option>
          </select>
        </FormField>
      </div>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Código</TableHead>
            <TableHead>Quarto</TableHead>
            <TableHead>Hóspede</TableHead>
            <TableHead>Saída prevista</TableHead>
            <TableHead>Saldo</TableHead>
            <TableHead>Avisos</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {data && data.items.length === 0 && (
            <TableRow>
              <TableCell colSpan={6} className="py-8 text-center text-muted-foreground">
                {status === "in_house" ? "Nenhuma hospedagem em andamento." : "Nenhuma hospedagem encerrada encontrada."}
              </TableCell>
            </TableRow>
          )}
          {data?.items.map(({ stay, balance_cents, minutes_late, risk }) => (
            <TableRow key={stay.id}>
              <TableCell>
                <Link to={`/hospedagens/${stay.id}`} className="font-semibold underline">
                  {stay.code}
                </Link>
                <span className="block text-xs text-muted-foreground">
                  <span aria-hidden="true">{stayStatusIcon[stay.status]} </span>
                  {stayStatusLabel[stay.status]}
                </span>
              </TableCell>
              <TableCell>{stay.room_code}</TableCell>
              <TableCell>{stay.guest_name}</TableCell>
              <TableCell>{`${formatDate(stay.check_out_date)} ${stay.expected_check_out_time}`}</TableCell>
              <TableCell>{formatMoney(balance_cents)}</TableCell>
              <TableCell>
                <span className="grid gap-1 text-xs font-medium text-destructive">
                  {minutes_late > 0 && <span>{`⚠ Saída atrasada há ${minutes_late} min`}</span>}
                  {risk && <span>{`⚠ Risco para a próxima reserva ${risk.next_code} (≈ ${risk.overrun_minutes} min)`}</span>}
                </span>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      {data && data.total > PAGE_SIZE && (
        <p className="mt-2 text-sm text-muted-foreground">{`Mostrando as ${PAGE_SIZE} primeiras de ${data.total}. Use a busca para refinar.`}</p>
      )}
    </section>
  );
}
