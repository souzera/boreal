import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { Link, useLocation, useParams } from "react-router-dom";

import { FormField } from "@/components/FormField";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import {
  applyMaintenanceUnavailability,
  completeMaintenance,
  getMaintenance,
  requestMaintenanceResolution,
  resolveMaintenanceConflict,
  startMaintenance,
  type ApiError,
} from "@/lib/api";
import { blockStatusIcon, blockStatusLabel, maintenanceStatusIcon, maintenanceStatusLabel, orderPriorityLabel, periodText } from "@/lib/blocks";
import { formatInstant } from "@/lib/format";
import { useSessionStore } from "@/stores/session";

function Line({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-wrap justify-between gap-2">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="font-medium">{children}</dd>
    </div>
  );
}

/** Uma ordem de manutenção: reservas afetadas, bloqueio do quarto e o próximo passo. */
export default function MaintenanceOrderPage() {
  const { id } = useParams();
  const orderId = Number(id);
  const token = useSessionStore((s) => s.token) ?? "";
  const location = useLocation();
  const queryClient = useQueryClient();
  const [notice, setNotice] = useState<string | null>((location.state as { notice?: string } | null)?.notice ?? null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [noteFor, setNoteFor] = useState<number | null>(null);
  const [note, setNote] = useState("");
  const [noteError, setNoteError] = useState<string | null>(null);
  const { data, isError } = useQuery({ queryKey: ["maintenance-order", orderId], queryFn: () => getMaintenance(token, orderId) });

  if (isError) {
    return (
      <section>
        <PageHeader title="Ordem de manutenção" description="Não foi possível carregar a ordem." />
        <Button render={<Link to="/manutencao" />} variant="outline">
          Voltar para a manutenção
        </Button>
      </section>
    );
  }
  if (!data) return null;
  const { order, conflicts, block } = data;
  const openConflicts = conflicts.filter((c) => c.status === "open");

  const refresh = async () => {
    await Promise.all(
      ["maintenance-order", "maintenance", "blocks", "room-map", "room-status"].map((key) => queryClient.invalidateQueries({ queryKey: [key] })),
    );
  };

  const run = async (action: () => Promise<unknown>, success: (result: unknown) => string) => {
    setNotice(null);
    setError(null);
    setBusy(true);
    try {
      const result = await action();
      setNotice(success(result));
    } catch (e) {
      setError((e as ApiError).message);
    } finally {
      setBusy(false);
      await refresh();
    }
  };

  const apply = () =>
    run(
      () => applyMaintenanceUnavailability(token, order.id, order.version),
      (result) => {
        const r = result as { still_open: unknown[]; effective: boolean };
        return r.effective
          ? "Indisponibilidade aplicada: o quarto está bloqueado."
          : `Ainda há ${r.still_open.length} reserva${r.still_open.length === 1 ? "" : "s"} afetada${r.still_open.length === 1 ? "" : "s"} para resolver.`;
      },
    );

  const saveNote = async (conflictId: number) => {
    if (!note.trim()) {
      setNoteError("Informe como a reserva foi resolvida.");
      return;
    }
    setNoteError(null);
    await run(() => resolveMaintenanceConflict(token, conflictId, note.trim()), () => "Resolução registrada.");
    setNoteFor(null);
    setNote("");
  };

  return (
    <section>
      <PageHeader
        title={`Ordem ${order.code}`}
        description={`Quarto ${order.room_code} · ${order.description}`}
        action={
          <Button variant="outline" render={<Link to="/manutencao" />}>
            Voltar para a manutenção
          </Button>
        }
      />
      {notice && (
        <p role="status" className="mb-3 rounded-lg border border-green-300 bg-green-50 p-2 text-sm text-green-900">
          {notice}
        </p>
      )}
      {error && (
        <p role="alert" className="mb-3 rounded-lg border border-destructive/40 bg-destructive/10 p-2 text-sm text-destructive">
          {error}
        </p>
      )}
      <dl className="mb-6 grid max-w-xl gap-1 text-sm">
        <Line label="Situação">
          <span aria-hidden="true">{maintenanceStatusIcon[order.status]} </span>
          {maintenanceStatusLabel[order.status]}
        </Line>
        <Line label="Prioridade">{orderPriorityLabel[order.priority]}</Line>
        <Line label="Período previsto">{periodText(order.planned_start, order.planned_end)}</Line>
        {order.actual_start && <Line label="Iniciada em">{formatInstant(order.actual_start)}</Line>}
        {order.actual_end && <Line label="Concluída em">{formatInstant(order.actual_end)}</Line>}
        {order.responsible && <Line label="Responsável">{order.responsible}</Line>}
        {order.notes && <Line label="Observações">{order.notes}</Line>}
        {block && (
          <Line label="Quarto">
            <span aria-hidden="true">{blockStatusIcon[block.status]} </span>
            {blockStatusLabel[block.status]}
          </Line>
        )}
      </dl>

      {conflicts.length > 0 && (
        <>
          <h2 className="mb-2 text-lg font-semibold">Reservas afetadas</h2>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Reserva</TableHead>
                <TableHead>Hóspede</TableHead>
                <TableHead>Período</TableHead>
                <TableHead>Situação</TableHead>
                <TableHead>Ações</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {conflicts.map((c) => (
                <TableRow key={c.id}>
                  <TableCell>
                    <Link className="underline" to={`/reservas/${c.reservation_id}`}>
                      {c.reservation_code}
                    </Link>
                  </TableCell>
                  <TableCell>{c.guest_name}</TableCell>
                  <TableCell>{periodText(c.check_in_date, c.check_out_date)}</TableCell>
                  <TableCell>
                    {c.status === "open" ? "✖ A resolver" : `✓ Resolvida${c.resolution_note ? `: ${c.resolution_note}` : ""}`}
                  </TableCell>
                  <TableCell>
                    {c.status === "open" &&
                      (noteFor === c.id ? (
                        <div className="grid gap-2">
                          <FormField id={`note-${c.id}`} label="Como foi resolvido" required error={noteError ?? undefined}>
                            <Input id={`note-${c.id}`} value={note} onChange={(e) => setNote(e.target.value)} />
                          </FormField>
                          <Button size="sm" disabled={busy} onClick={() => saveNote(c.id)}>
                            Registrar resolução
                          </Button>
                        </div>
                      ) : (
                        <Button size="sm" variant="outline" onClick={() => setNoteFor(c.id)}>
                          Registrar resolução
                        </Button>
                      ))}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </>
      )}

      {order.status === "completed" ? (
        <div className="mt-6 grid max-w-xl gap-2">
          <p className="text-sm">
            A manutenção foi concluída, mas o quarto continua bloqueado até ser liberado por quem tem autorização.
          </p>
          <Button variant="outline" render={<Link to="/bloqueios" />} className="w-fit">
            Ir para os bloqueios
          </Button>
        </div>
      ) : (
        <div className="mt-6 flex flex-wrap items-center gap-2">
          {order.status === "open" && (
            <>
              {conflicts.length > 0 && (
                <Button variant="outline" disabled={busy} onClick={apply}>
                  Aplicar indisponibilidade
                </Button>
              )}
              <Button disabled={busy || openConflicts.length > 0} onClick={() => run(() => startMaintenance(token, order.id, order.version), () => "Manutenção iniciada.")}>
                Iniciar manutenção
              </Button>
              {openConflicts.length > 0 && (
                <p className="text-sm text-muted-foreground">Resolva as reservas afetadas antes de iniciar a manutenção.</p>
              )}
            </>
          )}
          {order.status === "in_progress" && (
            <Button
              disabled={busy}
              onClick={() => run(() => requestMaintenanceResolution(token, order.id, order.version), () => "Ordem aguardando resolução.")}
            >
              Marcar como aguardando resolução
            </Button>
          )}
          {order.status === "awaiting_resolution" && (
            <Button disabled={busy} onClick={() => run(() => completeMaintenance(token, order.id, order.version), () => "Manutenção concluída. O quarto continua bloqueado até ser liberado.")}>
              Concluir manutenção
            </Button>
          )}
        </div>
      )}
    </section>
  );
}
