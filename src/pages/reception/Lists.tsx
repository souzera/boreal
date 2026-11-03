import { Link } from "react-router-dom";

import Icon from "@/components/Icon";
import { Button } from "@/components/ui/button";
import type { ArrivalItem, DepartureItem, RiskItem } from "@/lib/api";
import { severityClass, severityIcon, severityLabel } from "@/lib/alerts";
import { formatDate } from "@/lib/format";
import { balanceLabel } from "@/lib/stay";
import { cn } from "@/lib/utils";

const row = "grid gap-2 rounded-xl border bg-card p-3 text-sm shadow-card";
const tag = "inline-flex items-center gap-1 rounded-md border px-2 py-0.5 text-xs font-medium";

export function Empty({ children }: { children: string }) {
  return <p className="text-sm text-muted-foreground">{children}</p>;
}

/** "Mostrando 50 de 55" quando a lista passa do limite. */
export function More({ shown, total, to, label }: { shown: number; total: number; to: string; label: string }) {
  if (total <= shown) return null;
  return (
    <p className="text-sm text-muted-foreground">
      {`Mostrando ${shown} de ${total}. `}
      <Link to={to} className="underline underline-offset-2">
        {label}
      </Link>
    </p>
  );
}

export function ArrivalRow({ item, onDetails }: { item: ArrivalItem; onDetails: () => void }) {
  return (
    <li className={row}>
      <div className="flex flex-wrap items-center gap-2">
        <strong>{item.guest_name}</strong>
        <span>{`Quarto ${item.room_code} · ${formatDate(item.check_in_date)} às ${item.expected_time}`}</span>
        {item.overdue && (
          <span className={cn(tag, severityClass.high)}>
            <Icon name="schedule" size="xs" />
            Atrasada
          </span>
        )}
        {item.deposit_overdue && (
          <span className={cn(tag, severityClass.high)}>
            <Icon name="warning" size="xs" />
            Sinal vencido
          </span>
        )}
      </div>
      <div className="flex flex-wrap gap-2">
        <Button size="sm" render={<Link to={`/reservas/${item.reservation_id}?abrir=checkin`} />}>
          Confirmar check-in
        </Button>
        <Button size="sm" variant="outline" onClick={onDetails}>
          {`Ver detalhes da reserva ${item.code}`}
        </Button>
      </div>
    </li>
  );
}

export function DepartureRow({ item, onDetails }: { item: DepartureItem; onDetails: () => void }) {
  return (
    <li className={row}>
      <div className="flex flex-wrap items-center gap-2">
        <strong>{item.guest_name}</strong>
        <span>{`Quarto ${item.room_code} · saída ${formatDate(item.check_out_date)} às ${item.expected_time}`}</span>
        {item.minutes_late > 0 && (
          <span className={cn(tag, severityClass.high)}>
            <Icon name="schedule" size="xs" />
            {`Atrasada há ${item.minutes_late} min`}
          </span>
        )}
      </div>
      <p>{balanceLabel(item.balance_cents)}</p>
      <div className="flex flex-wrap gap-2">
        {item.balance_cents > 0 && (
          <Button size="sm" variant="outline" render={<Link to={`/hospedagens/${item.stay_id}?abrir=pagamento`} />}>
            Registrar pagamento
          </Button>
        )}
        <Button size="sm" render={<Link to={`/hospedagens/${item.stay_id}?abrir=checkout`} />}>
          Concluir check-out
        </Button>
        <Button size="sm" variant="outline" onClick={onDetails}>
          {`Ver detalhes da hospedagem ${item.code}`}
        </Button>
      </div>
    </li>
  );
}

export function RiskRow({ item }: { item: RiskItem }) {
  return (
    <li className={row}>
      <div className="flex flex-wrap items-center gap-2">
        <span className={cn(tag, severityClass.critical)}>
          <Icon name={severityIcon.critical} size="xs" />
          {severityLabel.critical}
        </span>
        <strong>{item.title}</strong>
      </div>
      <p className="text-muted-foreground">{item.detail}</p>
      <Button size="sm" variant="outline" className="w-fit" render={<Link to={item.link} />}>
        Abrir registro
      </Button>
    </li>
  );
}
