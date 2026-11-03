import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";

import { SidePanel } from "@/components/SidePanel";
import { Button } from "@/components/ui/button";
import { getStay } from "@/lib/api";
import { formatDate } from "@/lib/format";
import { balanceLabel } from "@/lib/stay";
import { useSessionStore } from "@/stores/session";
import { PanelError, PanelLine } from "./PanelState";

export function StayPanel({ id, onClose }: { id: number; onClose: () => void }) {
  const token = useSessionStore((s) => s.token) ?? "";
  const { data, isError } = useQuery({ queryKey: ["stay", id], queryFn: () => getStay(token, id) });
  const stay = data?.stay;
  const inHouse = stay?.status === "in_house";

  return (
    <SidePanel title={stay ? `Hospedagem ${stay.code}` : "Hospedagem"} onClose={onClose}>
      {isError && <PanelError />}
      {stay && data && (
        <div className="grid gap-3">
          <dl className="grid gap-1 text-sm">
            <PanelLine label="Situação">{inHouse ? "Hospedado" : "Check-out feito"}</PanelLine>
            <PanelLine label="Hóspede">{stay.guest_name}</PanelLine>
            <PanelLine label="Quarto">{stay.room_code}</PanelLine>
            <PanelLine label="Saída prevista">{`${formatDate(stay.check_out_date)} às ${stay.expected_check_out_time}`}</PanelLine>
            <PanelLine label="Conta">{balanceLabel(data.account.balance_cents)}</PanelLine>
          </dl>
          <div className="flex flex-wrap gap-2">
            {inHouse && <Button render={<Link to={`/hospedagens/${stay.id}?abrir=pagamento`} />}>Registrar pagamento</Button>}
            {inHouse && (
              <Button variant="outline" render={<Link to={`/hospedagens/${stay.id}?abrir=checkout`} />}>
                Concluir check-out
              </Button>
            )}
            <Button variant="outline" render={<Link to={`/hospedagens/${stay.id}`} />}>
              Abrir hospedagem completa
            </Button>
          </div>
        </div>
      )}
    </SidePanel>
  );
}
