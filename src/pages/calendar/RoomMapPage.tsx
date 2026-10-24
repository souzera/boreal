import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { useNavigate } from "react-router-dom";

import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { roomMap, type ApiError, type RoomMapCell } from "@/lib/api";
import { formatDate, toLocalIsoDate } from "@/lib/format";
import { statusIcon, statusLabel } from "@/lib/reservation";
import { cn } from "@/lib/utils";
import { useSessionStore } from "@/stores/session";

const DAYS = 7;

/** Soma dias a uma data `AAAA-MM-DD` sem depender do fuso (meio-dia evita virada por horário de verão). */
function addDays(iso: string, days: number): string {
  const [y, m, d] = iso.split("-").map(Number);
  return toLocalIsoDate(new Date(y, m - 1, d + days, 12));
}

const weekday = ["dom", "seg", "ter", "qua", "qui", "sex", "sáb"];

function dayHeader(iso: string): string {
  const [y, m, d] = iso.split("-").map(Number);
  return `${weekday[new Date(y, m - 1, d, 12).getDay()]} ${String(d).padStart(2, "0")}/${String(m).padStart(2, "0")}`;
}

/** Mapa quarto × dia: quem está onde, em texto (a situação nunca depende só de cor). */
export default function RoomMapPage() {
  const token = useSessionStore((s) => s.token) ?? "";
  const navigate = useNavigate();
  const today = toLocalIsoDate(new Date());
  const [from, setFrom] = useState(today);

  const { data, error } = useQuery({ queryKey: ["room-map", from], queryFn: () => roomMap(token, from, DAYS) });
  const days = Array.from({ length: DAYS }, (_, i) => addDays(from, i));

  const cellsOf = (roomId: number, date: string): RoomMapCell[] =>
    data?.cells.filter((c) => c.room_id === roomId && c.date === date) ?? [];

  return (
    <section>
      <PageHeader title="Mapa de quartos" description="Ocupação dos quartos por dia. Clique numa reserva para abri-la ou num dia livre para reservar." />
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
        <div className="overflow-x-auto">
          <table className="w-full border-collapse text-sm">
            <thead>
              <tr>
                <th scope="col" className="border p-2 text-left">
                  Quarto
                </th>
                {days.map((d) => (
                  <th key={d} scope="col" className="border p-2 text-left font-medium">
                    {dayHeader(d)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {data.rooms.map((room) => (
                <tr key={room.id} className={cn(!room.active && "opacity-60")}>
                  <th scope="row" className="border p-2 text-left font-medium">
                    {`${room.code} · ${room.category_name}${room.active ? "" : " (inativo)"}`}
                  </th>
                  {days.map((date) => {
                    const cells = cellsOf(room.id, date);
                    const canBook = room.active && date >= today && cells.every((c) => c.is_check_out);
                    return (
                      <td key={date} className="min-w-28 border p-1 align-top">
                        <div className="grid gap-1">
                          {cells.map((c) => (
                            <button
                              key={c.reservation_id}
                              type="button"
                              className="rounded border bg-muted/50 px-2 py-1 text-left hover:bg-muted"
                              onClick={() => navigate(`/reservas/${c.reservation_id}`)}
                            >
                              <span aria-hidden="true">{statusIcon[c.status]} </span>
                              {`${c.guest_name} · ${statusLabel[c.status]}${c.is_check_in ? " · chegada" : ""}${c.is_check_out ? " · saída" : ""}`}
                            </button>
                          ))}
                          {canBook && (
                            <button
                              type="button"
                              aria-label={`Reservar quarto ${room.code} em ${formatDate(date)}`}
                              className="rounded border border-dashed px-2 py-1 text-left text-muted-foreground hover:bg-muted"
                              onClick={() => navigate(`/reservas/nova?room=${room.id}&date=${date}`)}
                            >
                              Livre
                            </button>
                          )}
                        </div>
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
