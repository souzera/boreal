import { useQuery } from "@tanstack/react-query";
import { useNavigate, useSearchParams } from "react-router-dom";

import { PageHeader } from "@/components/PageHeader";
import { RoomMapGrid, addDays } from "@/components/RoomMapGrid";
import { Button } from "@/components/ui/button";
import { roomMap, type ApiError } from "@/lib/api";
import { formatDate, toLocalIsoDate } from "@/lib/format";
import { ROOM_STATE_ORDER, ROOM_STATE_LABEL, ROOM_STATE_STYLE } from "@/lib/roomState";
import { useSessionStore } from "@/stores/session";
import Icon from "@/components/Icon";

const DAYS = 7;

/** Mapa quarto × dia: quem está onde, em texto (a situação nunca depende só de cor). */
export default function RoomMapPage() {
  const token = useSessionStore((s) => s.token) ?? "";
  const navigate = useNavigate();
  const today = toLocalIsoDate(new Date());
  const [params, setParams] = useSearchParams();
  const fromParam = params.get("de");
  const from = fromParam && /^\d{4}-\d{2}-\d{2}$/.test(fromParam) ? fromParam : today;
  // A data fica no endereço: abrir a fila de limpeza e voltar não perde a agenda.
  const setFrom = (date: string) => setParams({ de: date }, { replace: true });

  const { data, error } = useQuery({ queryKey: ["room-map", from], queryFn: () => roomMap(token, from, DAYS) });
  return (
    <section>
      <PageHeader title="Mapa de quartos" description="Ocupação dos quartos por dia. Clique numa reserva para abri-la ou num dia livre para reservar." />
      <ul aria-label="Legenda dos estados do quarto" className="mb-3 flex flex-wrap gap-2 text-xs">
        {ROOM_STATE_ORDER.map((state) => (
          <li key={state} className={`rounded border px-2 py-0.5 ${ROOM_STATE_STYLE[state].className}`}>
            <Icon name={ROOM_STATE_STYLE[state].icon} size="xs" className="mr-1 align-middle" />
            {ROOM_STATE_LABEL[state]}
          </li>
        ))}
      </ul>
      <div className="mb-4 flex flex-wrap items-center gap-2">
        <Button variant="outline" onClick={() => setFrom(addDays(from, -DAYS))}>
          Período anterior
        </Button>
        <Button variant="outline" onClick={() => setFrom(today)}>
          Hoje
        </Button>
        <Button variant="outline" onClick={() => setFrom(addDays(from, DAYS))}>
          Próximo período
        </Button>
        <span className="text-sm text-muted-foreground">{`${formatDate(from)} a ${formatDate(addDays(from, DAYS - 1))}`}</span>
      </div>
      {error && (
        <p role="alert" className="mb-4 text-sm text-destructive">
          {(error as unknown as ApiError).message}
        </p>
      )}
      {data && (
        <RoomMapGrid
          data={data}
          from={from}
          days={DAYS}
          today={today}
          onOpenReservation={(id) => navigate(`/reservas/${id}`)}
          onBook={(roomId, date) => navigate(`/reservas/nova?room=${roomId}&date=${date}`)}
        />
      )}
    </section>
  );
}
