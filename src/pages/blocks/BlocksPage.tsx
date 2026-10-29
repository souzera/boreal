import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { Link } from "react-router-dom";

import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { usePermissions } from "@/hooks/usePermissions";
import { listBlocks, type RoomBlock } from "@/lib/api";
import { blockStatusIcon, blockStatusLabel, periodText } from "@/lib/blocks";
import { useSessionStore } from "@/stores/session";
import BlockDialog from "./BlockDialog";
import ReleaseDialog from "./ReleaseDialog";

/** Bloqueios de quarto: por período ou até a liberação manual. */
export default function BlocksPage() {
  const token = useSessionStore((s) => s.token) ?? "";
  const { can } = usePermissions();
  const queryClient = useQueryClient();
  const [includeReleased, setIncludeReleased] = useState(false);
  const [creating, setCreating] = useState(false);
  const [releasing, setReleasing] = useState<RoomBlock | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const { data } = useQuery({ queryKey: ["blocks", includeReleased], queryFn: () => listBlocks(token, includeReleased) });
  const canRelease = can("release_room");

  const done = async (message: string) => {
    setCreating(false);
    setReleasing(null);
    setNotice(message);
    await Promise.all(["blocks", "room-map", "room-status"].map((key) => queryClient.invalidateQueries({ queryKey: [key] })));
  };

  return (
    <section>
      <PageHeader
        title="Bloqueios"
        description="Quartos indisponíveis por período ou até serem liberados."
        action={canRelease ? <Button onClick={() => setCreating(true)}>Criar bloqueio</Button> : undefined}
      />
      {!canRelease && (
        <p className="mb-3 text-sm text-muted-foreground">
          Criar e encerrar bloqueios precisa da permissão de liberar quartos. Peça a quem administra o sistema.
        </p>
      )}
      {notice && (
        <p role="status" className="mb-3 rounded-lg border border-green-300 bg-green-50 p-2 text-sm text-green-900">
          {notice}
        </p>
      )}
      <label className="mb-4 flex items-center gap-2 text-sm">
        <input type="checkbox" checked={includeReleased} onChange={(e) => setIncludeReleased(e.target.checked)} />
        Mostrar bloqueios já liberados
      </label>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Quarto</TableHead>
            <TableHead>Período</TableHead>
            <TableHead>Motivo</TableHead>
            <TableHead>Situação</TableHead>
            <TableHead>Ações</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {data && data.length === 0 && (
            <TableRow>
              <TableCell colSpan={5} className="py-8 text-center text-muted-foreground">
                Nenhum bloqueio.
              </TableCell>
            </TableRow>
          )}
          {data?.map((b) => (
            <TableRow key={b.id}>
              <TableCell className="font-medium">{b.room_code}</TableCell>
              <TableCell>{periodText(b.start_date, b.end_date)}</TableCell>
              <TableCell>{b.reason}</TableCell>
              <TableCell>
                <span aria-hidden="true">{blockStatusIcon[b.status]} </span>
                {blockStatusLabel[b.status]}
              </TableCell>
              <TableCell>
                <div className="flex flex-wrap gap-2">
                  {b.maintenance_order_id !== null && (
                    <Button size="sm" variant="outline" render={<Link to={`/manutencao/${b.maintenance_order_id}`} />}>
                      Ver ordem de manutenção
                    </Button>
                  )}
                  {canRelease && b.status !== "released" && (
                    <Button size="sm" onClick={() => setReleasing(b)}>
                      Liberar quarto
                    </Button>
                  )}
                </div>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      <BlockDialog open={creating} onClose={() => setCreating(false)} onDone={done} />
      <ReleaseDialog open={releasing !== null} block={releasing} onClose={() => setReleasing(null)} onDone={done} />
    </section>
  );
}
