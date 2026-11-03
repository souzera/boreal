import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";

import { SidePanel } from "@/components/SidePanel";
import { Button } from "@/components/ui/button";
import { getGuest } from "@/lib/api";
import { useSessionStore } from "@/stores/session";
import { PanelError, PanelLine } from "./PanelState";

export function GuestPanel({ id, onClose }: { id: number; onClose: () => void }) {
  const token = useSessionStore((s) => s.token) ?? "";
  const { data: guest, isError } = useQuery({ queryKey: ["guest", id], queryFn: () => getGuest(token, id) });

  return (
    <SidePanel title={guest ? guest.full_name : "Hóspede"} onClose={onClose}>
      {isError && <PanelError />}
      {guest && (
        <div className="grid gap-3">
          <dl className="grid gap-1 text-sm">
            <PanelLine label="Código">{guest.code}</PanelLine>
            <PanelLine label="Telefone">{guest.phone || "Não informado"}</PanelLine>
            <PanelLine label="Cadastro">{guest.active ? "Ativo" : "Inativo"}</PanelLine>
          </dl>
          <Button variant="outline" className="w-fit" render={<Link to={`/hospedes/${guest.id}`} />}>
            Abrir cadastro completo
          </Button>
        </div>
      )}
    </SidePanel>
  );
}
