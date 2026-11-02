import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";

import { RoomStateBadge } from "@/components/RoomStateBadge";
import { SidePanel } from "@/components/SidePanel";
import { Button } from "@/components/ui/button";
import { listRoomStatus } from "@/lib/api";
import { useSessionStore } from "@/stores/session";
import { PanelError, PanelLine } from "./PanelState";

export function RoomPanel({ id, onClose }: { id: number; onClose: () => void }) {
  const token = useSessionStore((s) => s.token) ?? "";
  const { data, isError } = useQuery({ queryKey: ["room-status"], queryFn: () => listRoomStatus(token) });
  const room = data?.find((r) => r.room_id === id);
  const needsCleaning = room?.state === "awaiting_cleaning" || room?.state === "cleaning";

  return (
    <SidePanel title={room ? `Quarto ${room.code}` : "Quarto"} onClose={onClose}>
      {(isError || (data && !room)) && <PanelError />}
      {room && (
        <div className="grid gap-3">
          <dl className="grid gap-1 text-sm">
            <PanelLine label="Categoria">{room.category_name}</PanelLine>
            <PanelLine label="Situação">
              <RoomStateBadge state={room.state} freeAt={room.free_at} />
            </PanelLine>
          </dl>
          {needsCleaning && (
            <Button variant="outline" className="w-fit" render={<Link to="/limpeza" />}>
              Ver na fila de limpeza
            </Button>
          )}
        </div>
      )}
    </SidePanel>
  );
}
