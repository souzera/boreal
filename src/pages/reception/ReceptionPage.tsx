import { useQuery } from "@tanstack/react-query";
import { Link, useNavigate, useSearchParams } from "react-router-dom";

import Icon from "@/components/Icon";
import { RoomMapGrid } from "@/components/RoomMapGrid";
import { useAlertSummary } from "@/hooks/useAlertSummary";
import { useReceptionOverview } from "@/hooks/useReceptionOverview";
import { cleaningQueue, roomMap } from "@/lib/api";
import { severityClass, severityIcon, severityLabel } from "@/lib/alerts";
import { formatDate, toLocalIsoDate } from "@/lib/format";
import { cn } from "@/lib/utils";
import { useSessionStore } from "@/stores/session";
import { ArrivalRow, DepartureRow, Empty, More, RiskRow } from "./Lists";
import { GuestPanel } from "./panel/GuestPanel";
import { ReservationPanel } from "./panel/ReservationPanel";
import { RoomPanel } from "./panel/RoomPanel";
import { StayPanel } from "./panel/StayPanel";
import { SummaryStrip } from "./SummaryStrip";

const DAYS = 7;
const TABS = [
  { key: "chegadas", label: "Chegadas" },
  { key: "saidas", label: "Saídas" },
  { key: "risco", label: "Em risco" },
] as const;
type Tab = (typeof TABS)[number]["key"];

const PANEL = /^(reserva|hospedagem|hospede|quarto)-(\d+)$/;

/** Tela inicial: o trabalho de hoje (chegadas, saídas, riscos) ao lado do mapa de 7 dias. */
export default function ReceptionPage() {
  const token = useSessionStore((s) => s.token) ?? "";
  const navigate = useNavigate();
  const [params, setParams] = useSearchParams();
  const today = toLocalIsoDate(new Date());

  const aba = params.get("aba");
  const tab: Tab = TABS.some((t) => t.key === aba) ? (aba as Tab) : "chegadas";
  const panelMatch = PANEL.exec(params.get("painel") ?? "");
  const panel = panelMatch ? { kind: panelMatch[1], id: Number(panelMatch[2]) } : null;

  const { data: overview, isError } = useReceptionOverview();
  const { data: alerts } = useAlertSummary();
  const { data: map } = useQuery({ queryKey: ["room-map", today, DAYS], queryFn: () => roomMap(token, today, DAYS), enabled: !!token });
  const { data: queue } = useQuery({ queryKey: ["cleaning-queue"], queryFn: () => cleaningQueue(token), enabled: !!token });

  const update = (patch: Record<string, string | null>) => {
    const next = new URLSearchParams(params);
    for (const [key, value] of Object.entries(patch)) {
      if (value === null) next.delete(key);
      else next.set(key, value);
    }
    setParams(next);
  };
  const selectTab = (key: Tab) => update({ aba: key, painel: null });
  const openPanel = (value: string) => update({ painel: value });
  const closePanel = () => update({ painel: null });

  const panelView = panel && (
    <>
      {panel.kind === "reserva" && <ReservationPanel id={panel.id} onClose={closePanel} />}
      {panel.kind === "hospedagem" && <StayPanel id={panel.id} onClose={closePanel} />}
      {panel.kind === "hospede" && <GuestPanel id={panel.id} onClose={closePanel} />}
      {panel.kind === "quarto" && <RoomPanel id={panel.id} onClose={closePanel} />}
    </>
  );

  return (
    <section>
      <h1 className="text-[26px] font-[650] leading-tight">Recepção</h1>
      <p className="mb-4 text-sm text-muted-foreground">Hoje, {formatDate(today)}</p>

      {isError && (
        <p role="alert" className="mb-4 text-sm text-destructive">
          Não foi possível carregar a Recepção. Tente de novo em instantes.
        </p>
      )}
      {overview && <SummaryStrip summary={overview.summary} onSelectTab={selectTab} />}

      <div className="grid gap-6 lg:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
        <section aria-label="Mapa dos próximos 7 dias" className="min-w-0">
          <h2 className="mb-2 text-lg font-semibold">Próximos 7 dias</h2>
          {map && (
            <RoomMapGrid
              data={map}
              from={today}
              days={DAYS}
              today={today}
              onOpenReservation={(id) => openPanel(`reserva-${id}`)}
              onBook={(roomId, date) => navigate(`/reservas/nova?room=${roomId}&date=${date}`)}
              onOpenRoom={(id) => openPanel(`quarto-${id}`)}
            />
          )}
          <Link to="/mapa" className="mt-2 inline-block text-sm underline underline-offset-2">
            Abrir o mapa de quartos completo
          </Link>
        </section>

        <div className="min-w-0">
          {panelView ?? (
            <section aria-label="Trabalho de hoje">
              <div role="tablist" aria-label="Trabalho de hoje" className="mb-3 flex gap-2">
                {TABS.map((t) => (
                  <button
                    key={t.key}
                    type="button"
                    role="tab"
                    id={`aba-${t.key}`}
                    aria-selected={tab === t.key}
                    aria-controls="aba-conteudo"
                    className={cn("rounded-lg border px-3 py-1.5 text-sm font-medium", tab === t.key ? "bg-primary text-primary-foreground" : "hover:bg-muted")}
                    onClick={() => selectTab(t.key)}
                  >
                    {t.label}
                  </button>
                ))}
              </div>
              <div id="aba-conteudo" role="tabpanel" aria-labelledby={`aba-${tab}`} className="grid gap-2">
                {!overview && !isError && <Empty>Carregando…</Empty>}
                {overview && tab === "chegadas" && (
                  <>
                    {overview.arrivals.items.length === 0 && <Empty>Nenhuma chegada pendente para hoje.</Empty>}
                    <ul className="grid gap-2">
                      {overview.arrivals.items.map((a) => (
                        <ArrivalRow key={a.reservation_id} item={a} onDetails={() => openPanel(`reserva-${a.reservation_id}`)} />
                      ))}
                    </ul>
                    <More shown={overview.arrivals.items.length} total={overview.arrivals.total} to="/reservas" label="Ver todas as reservas" />
                  </>
                )}
                {overview && tab === "saidas" && (
                  <>
                    {overview.departures.items.length === 0 && <Empty>Nenhuma saída pendente para hoje.</Empty>}
                    <ul className="grid gap-2">
                      {overview.departures.items.map((d) => (
                        <DepartureRow key={d.stay_id} item={d} onDetails={() => openPanel(`hospedagem-${d.stay_id}`)} />
                      ))}
                    </ul>
                    <More shown={overview.departures.items.length} total={overview.departures.total} to="/hospedagens" label="Ver todas as hospedagens" />
                  </>
                )}
                {overview && tab === "risco" && (
                  <>
                    {overview.at_risk.items.length === 0 && <Empty>Nenhum quarto ou reserva em risco agora.</Empty>}
                    <ul className="grid gap-2">
                      {overview.at_risk.items.map((r, i) => (
                        <RiskRow key={`${r.link}-${i}`} item={r} />
                      ))}
                    </ul>
                    <More shown={overview.at_risk.items.length} total={overview.at_risk.total} to="/alertas" label="Ver todos os alertas" />
                  </>
                )}
              </div>
            </section>
          )}
        </div>
      </div>

      <section aria-label="Alertas" className="mt-6 grid max-w-2xl gap-2">
        <h2 className="text-lg font-semibold">Alertas urgentes</h2>
        {alerts && alerts.top.length === 0 && <p className="text-sm text-muted-foreground">Nenhum alerta ativo. Está tudo em ordem.</p>}
        {alerts && alerts.top.length > 0 && (
          <ul className="grid gap-2">
            {alerts.top.slice(0, 3).map((a) => (
              <li key={a.id}>
                <Link to={a.link} className="grid gap-0.5 rounded-xl border bg-card p-3 text-sm shadow-card hover:bg-muted">
                  <span className="flex flex-wrap items-center gap-2">
                    <span className={cn("inline-flex items-center gap-1 rounded-md border px-2 py-0.5 text-xs font-medium", severityClass[a.severity])}>
                      <Icon name={severityIcon[a.severity]} size="xs" />
                      {severityLabel[a.severity]}
                    </span>
                    <strong>{a.title}</strong>
                  </span>
                  <span className="text-muted-foreground">{a.detail}</span>
                </Link>
              </li>
            ))}
          </ul>
        )}
        <Link to="/alertas" className="w-fit text-sm underline underline-offset-2">
          Ver todos os alertas
        </Link>
      </section>

      {queue &&
        (queue.length > 0 ? (
          <Link to="/limpeza" className="mt-4 inline-block rounded-lg border px-3 py-2 text-sm font-medium underline-offset-2 hover:underline">
            {`Quartos aguardando limpeza: ${queue.length}`}
          </Link>
        ) : (
          <p className="mt-4 text-sm text-muted-foreground">Nenhum quarto aguardando limpeza.</p>
        ))}
    </section>
  );
}
