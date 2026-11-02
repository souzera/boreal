import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";

import Icon from "@/components/Icon";
import { SidePanel } from "@/components/SidePanel";
import { Button } from "@/components/ui/button";
import { getReservation } from "@/lib/api";
import { formatDate, formatMoney, toLocalIsoDate } from "@/lib/format";
import { isOpen, statusIcon, statusLabel } from "@/lib/reservation";
import { useSessionStore } from "@/stores/session";
import { PanelError, PanelLine } from "./PanelState";

export function ReservationPanel({ id, onClose }: { id: number; onClose: () => void }) {
  const token = useSessionStore((s) => s.token) ?? "";
  const { data, isError } = useQuery({ queryKey: ["reservation", id], queryFn: () => getReservation(token, id) });
  const r = data?.reservation;
  const canCheckIn = !!r && isOpen(r.status) && r.check_in_date <= toLocalIsoDate(new Date());

  return (
    <SidePanel title={r ? `Reserva ${r.code}` : "Reserva"} onClose={onClose}>
      {isError && <PanelError />}
      {r && (
        <div className="grid gap-3">
          <dl className="grid gap-1 text-sm">
            <PanelLine label="Situação">
              <Icon name={statusIcon[r.status]} size="xs" className="mr-1 align-middle" />
              {statusLabel[r.status]}
            </PanelLine>
            <PanelLine label="Hóspede">{r.guest_name}</PanelLine>
            <PanelLine label="Quarto">{r.room_code}</PanelLine>
            <PanelLine label="Período">{`${formatDate(r.check_in_date)} a ${formatDate(r.check_out_date)}`}</PanelLine>
            <PanelLine label="Total combinado">{formatMoney(r.agreed_total_cents)}</PanelLine>
            <PanelLine label="Sinal recebido">{`${formatMoney(r.deposit_paid_cents)} de ${formatMoney(r.deposit_expected_cents)}`}</PanelLine>
          </dl>
          <div className="flex flex-wrap gap-2">
            {canCheckIn && <Button render={<Link to={`/reservas/${r.id}?abrir=checkin`} />}>Confirmar check-in</Button>}
            <Button variant="outline" render={<Link to={`/reservas/${r.id}`} />}>
              Abrir reserva completa
            </Button>
          </div>
        </div>
      )}
    </SidePanel>
  );
}
