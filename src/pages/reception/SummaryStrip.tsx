import { Link } from "react-router-dom";

import type { ReceptionSummary } from "@/lib/api";
import { cn } from "@/lib/utils";

const card = "grid gap-0.5 rounded-xl border bg-card p-3 text-left shadow-card";

interface Props {
  summary: ReceptionSummary;
  onSelectTab: (tab: "chegadas" | "saidas") => void;
}

/** Faixa de números do dia: o que exige ação vem primeiro (chegadas, saídas, alertas críticos). */
export function SummaryStrip({ summary, onSelectTab }: Props) {
  return (
    <ul aria-label="Resumo do dia" className="mb-4 grid grid-cols-2 gap-3 md:grid-cols-5">
      <li>
        <button type="button" className={cn(card, "w-full hover:bg-muted")} onClick={() => onSelectTab("chegadas")}>
          <span className="text-sm text-muted-foreground">Chegadas</span>
          <strong className="text-2xl">{summary.arrivals}</strong>
        </button>
      </li>
      <li>
        <button type="button" className={cn(card, "w-full hover:bg-muted")} onClick={() => onSelectTab("saidas")}>
          <span className="text-sm text-muted-foreground">Saídas</span>
          <strong className="text-2xl">{summary.departures}</strong>
        </button>
      </li>
      <li className={card}>
        <span className="text-sm text-muted-foreground">Quartos ocupados</span>
        <strong className="text-2xl">{`${summary.occupied_rooms} de ${summary.total_rooms}`}</strong>
      </li>
      <li className={card}>
        <span className="text-sm text-muted-foreground">Quartos disponíveis</span>
        <strong className="text-2xl">{summary.available_rooms}</strong>
      </li>
      <li>
        <Link to="/alertas" className={cn(card, "hover:bg-muted")}>
          <span className="text-sm text-muted-foreground">Alertas críticos</span>
          <strong className="text-2xl">{summary.critical_alerts}</strong>
        </Link>
      </li>
    </ul>
  );
}
