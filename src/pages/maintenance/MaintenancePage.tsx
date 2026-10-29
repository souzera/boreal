import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";

import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { listMaintenance } from "@/lib/api";
import { maintenanceStatusIcon, maintenanceStatusLabel, orderPriorityLabel, periodText } from "@/lib/blocks";
import { useSessionStore } from "@/stores/session";
import OrderDialog from "./OrderDialog";

/** Ordens de manutenção: abertas por padrão, com opção de ver as concluídas. */
export default function MaintenancePage() {
  const token = useSessionStore((s) => s.token) ?? "";
  const navigate = useNavigate();
  const [onlyOpen, setOnlyOpen] = useState(true);
  const [creating, setCreating] = useState(false);
  const { data } = useQuery({ queryKey: ["maintenance", onlyOpen], queryFn: () => listMaintenance(token, onlyOpen) });

  return (
    <section>
      <PageHeader
        title="Manutenção"
        description="Ordens de manutenção dos quartos."
        action={<Button onClick={() => setCreating(true)}>Abrir ordem de manutenção</Button>}
      />
      <label className="mb-4 flex items-center gap-2 text-sm">
        <input type="checkbox" checked={!onlyOpen} onChange={(e) => setOnlyOpen(!e.target.checked)} />
        Mostrar ordens concluídas
      </label>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Código</TableHead>
            <TableHead>Quarto</TableHead>
            <TableHead>Descrição</TableHead>
            <TableHead>Prioridade</TableHead>
            <TableHead>Período previsto</TableHead>
            <TableHead>Situação</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {data && data.length === 0 && (
            <TableRow>
              <TableCell colSpan={6} className="py-8 text-center text-muted-foreground">
                Nenhuma ordem de manutenção.
              </TableCell>
            </TableRow>
          )}
          {data?.map((o) => (
            <TableRow key={o.id}>
              <TableCell>
                <Link className="font-medium underline" to={`/manutencao/${o.id}`}>
                  {o.code}
                </Link>
              </TableCell>
              <TableCell>{o.room_code}</TableCell>
              <TableCell>{o.description}</TableCell>
              <TableCell>{orderPriorityLabel[o.priority]}</TableCell>
              <TableCell>{periodText(o.planned_start, o.planned_end)}</TableCell>
              <TableCell>
                <span aria-hidden="true">{maintenanceStatusIcon[o.status]} </span>
                {maintenanceStatusLabel[o.status]}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      <OrderDialog
        open={creating}
        onClose={() => setCreating(false)}
        onDone={(opened) => {
          setCreating(false);
          navigate(`/manutencao/${opened.id}`, { state: { notice: opened.conflicts.length > 0 ? `Ordem ${opened.code} aberta. ${opened.conflicts.length} reserva(s) afetada(s) precisam ser resolvidas.` : `Ordem ${opened.code} aberta. O quarto já está bloqueado.` } });
        }}
      />
    </section>
  );
}
