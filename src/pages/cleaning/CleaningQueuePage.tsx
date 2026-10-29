import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import {
  cleaningQueue,
  completeCleaning,
  listRoomStatus,
  startCleaning,
  type ApiError,
  type CleaningItem,
} from "@/lib/api";
import { formatTime } from "@/lib/format";
import { formatLocalDateTime, PRIORITY_ICON, PRIORITY_LABEL } from "@/lib/roomState";
import { useSessionStore } from "@/stores/session";
import PriorityDialog from "./PriorityDialog";

const statusLabel = { awaiting: "Aguardando limpeza", in_progress: "Em limpeza", done: "Concluída" } as const;

/** Fila de limpeza: o que limpar primeiro, com ação específica por situação. */
export default function CleaningQueuePage() {
  const token = useSessionStore((s) => s.token) ?? "";
  const queryClient = useQueryClient();
  const { data } = useQuery({ queryKey: ["cleaning-queue"], queryFn: () => cleaningQueue(token) });
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<number | null>(null);
  const [adjusting, setAdjusting] = useState<CleaningItem | null>(null);

  const refresh = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["cleaning-queue"] }),
      queryClient.invalidateQueries({ queryKey: ["room-map"] }),
      queryClient.invalidateQueries({ queryKey: ["room-status"] }),
    ]);
  };

  const run = async (item: CleaningItem, action: () => Promise<unknown>, success: () => Promise<string>) => {
    setNotice(null);
    setError(null);
    setBusyId(item.task.id);
    try {
      await action();
      setNotice(await success());
    } catch (e) {
      setError((e as ApiError).message);
    } finally {
      setBusyId(null);
      await refresh();
    }
  };

  const start = (item: CleaningItem) =>
    run(item, () => startCleaning(token, item.task.id, item.task.version), async () => `Limpeza do quarto ${item.task.room_code} iniciada.`);

  const complete = (item: CleaningItem) =>
    run(
      item,
      () => completeCleaning(token, item.task.id, item.task.version),
      async () => {
        const base = `Limpeza do quarto ${item.task.room_code} concluída.`;
        try {
          const room = (await listRoomStatus(token)).find((r) => r.room_id === item.task.room_id);
          if (room?.state === "clean_waiting_interval" && room.free_at) return `${base} O quarto fica livre às ${room.free_at.slice(11, 16)}.`;
          if (room?.state === "available") return `${base} O quarto está disponível.`;
        } catch {
          // O resultado principal já foi salvo; a hora de liberação é só um complemento.
        }
        return base;
      },
    );

  return (
    <section>
      <PageHeader title="Limpeza" description="Quartos que precisam de limpeza, do mais urgente ao menos urgente." />
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
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Quarto</TableHead>
            <TableHead>Prioridade</TableHead>
            <TableHead>Situação</TableHead>
            <TableHead>Saiu às</TableHead>
            <TableHead>Próxima chegada</TableHead>
            <TableHead>Ações</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {data && data.length === 0 && (
            <TableRow>
              <TableCell colSpan={6} className="py-8 text-center text-muted-foreground">
                Nenhum quarto aguardando limpeza.
              </TableCell>
            </TableRow>
          )}
          {data?.map((item) => (
            <TableRow key={item.task.id}>
              <TableCell className="font-medium">{item.task.room_code}</TableCell>
              <TableCell>
                <span aria-hidden="true">{PRIORITY_ICON[item.priority]} </span>
                {PRIORITY_LABEL[item.priority]}
                {item.task.priority_override && <span className="text-xs text-muted-foreground"> (ajustada)</span>}
              </TableCell>
              <TableCell>{statusLabel[item.task.status]}</TableCell>
              <TableCell>{formatTime(item.task.ready_since)}</TableCell>
              <TableCell>{item.next_arrival ? formatLocalDateTime(item.next_arrival) : "Sem reserva"}</TableCell>
              <TableCell>
                <div className="flex flex-wrap gap-2">
                  {item.task.status === "awaiting" && (
                    <Button size="sm" disabled={busyId === item.task.id} onClick={() => start(item)}>
                      Iniciar limpeza
                    </Button>
                  )}
                  {item.task.status === "in_progress" && (
                    <Button size="sm" disabled={busyId === item.task.id} onClick={() => complete(item)}>
                      Concluir limpeza
                    </Button>
                  )}
                  <Button size="sm" variant="outline" onClick={() => setAdjusting(item)}>
                    Ajustar prioridade
                  </Button>
                </div>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      <PriorityDialog
        open={adjusting !== null}
        item={adjusting}
        onClose={() => setAdjusting(null)}
        onDone={async (message) => {
          setAdjusting(null);
          setNotice(message);
          await refresh();
        }}
      />
    </section>
  );
}
