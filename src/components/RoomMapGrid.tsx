import Icon from "@/components/Icon";
import { RoomStateBadge } from "@/components/RoomStateBadge";
import type { RoomMap, RoomMapCell } from "@/lib/api";
import { formatDate, toLocalIsoDate } from "@/lib/format";
import { statusIcon, statusLabel } from "@/lib/reservation";
import { cn } from "@/lib/utils";

/** Soma dias a uma data `AAAA-MM-DD` sem depender do fuso (meio-dia evita virada por horário de verão). */
export function addDays(iso: string, days: number): string {
  const [y, m, d] = iso.split("-").map(Number);
  return toLocalIsoDate(new Date(y, m - 1, d + days, 12));
}

const weekday = ["dom", "seg", "ter", "qua", "qui", "sex", "sáb"];

function dayHeader(iso: string): string {
  const [y, m, d] = iso.split("-").map(Number);
  return `${weekday[new Date(y, m - 1, d, 12).getDay()]} ${String(d).padStart(2, "0")}/${String(m).padStart(2, "0")}`;
}

interface Props {
  data: RoomMap;
  from: string;
  days: number;
  today: string;
  onOpenReservation: (reservationId: number) => void;
  /** Sem esta função a grade não oferece o botão "Livre". */
  onBook?: (roomId: number, date: string) => void;
  /** Quando informada, o código do quarto vira um botão que a chama. */
  onOpenRoom?: (roomId: number) => void;
}

/** Grade quarto × dia: quem está onde, em texto (a situação nunca depende só de cor). */
export function RoomMapGrid({ data, from, days, today, onOpenReservation, onBook, onOpenRoom }: Props) {
  const dates = Array.from({ length: days }, (_, i) => addDays(from, i));
  const cellsOf = (roomId: number, date: string): RoomMapCell[] =>
    data.cells.filter((c) => c.room_id === roomId && c.date === date);

  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-sm">
        <thead>
          <tr>
            <th scope="col" className="border p-2 text-left">
              Quarto
            </th>
            {dates.map((d) => (
              <th key={d} scope="col" className="border p-2 text-left font-medium">
                {dayHeader(d)}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {data.rooms.map((room) => {
            const label = `${room.code} · ${room.category_name}${room.active ? "" : " (inativo)"}`;
            return (
              <tr key={room.id} className={cn(!room.active && "opacity-60")}>
                <th scope="row" className="border p-2 text-left font-medium">
                  {onOpenRoom && room.active ? (
                    <button type="button" className="text-left underline-offset-2 hover:underline" onClick={() => onOpenRoom(room.id)}>
                      {label}
                    </button>
                  ) : (
                    label
                  )}
                  {room.state && (
                    <span className="mt-1 block">
                      <RoomStateBadge
                        state={room.state}
                        freeAt={room.free_at}
                        to={room.state === "awaiting_cleaning" || room.state === "cleaning" ? "/limpeza" : undefined}
                      />
                    </span>
                  )}
                </th>
                {dates.map((date) => {
                  const cells = cellsOf(room.id, date);
                  const canBook = !!onBook && room.active && date >= today && cells.every((c) => c.is_check_out);
                  return (
                    <td key={date} className="min-w-28 border p-1 align-top">
                      <div className="grid gap-1">
                        {cells.map((c) => (
                          <button
                            key={c.reservation_id}
                            type="button"
                            className="rounded border bg-muted/50 px-2 py-1 text-left hover:bg-muted"
                            onClick={() => onOpenReservation(c.reservation_id)}
                          >
                            <Icon name={statusIcon[c.status]} size="xs" className="mr-1 align-middle" />
                            {`${c.guest_name} · ${statusLabel[c.status]}${c.is_check_in ? " · chegada" : ""}${c.is_check_out ? " · saída" : ""}`}
                          </button>
                        ))}
                        {canBook && (
                          <button
                            type="button"
                            aria-label={`Reservar quarto ${room.code} em ${formatDate(date)}`}
                            className="rounded border border-dashed px-2 py-1 text-left text-muted-foreground hover:bg-muted"
                            onClick={() => onBook(room.id, date)}
                          >
                            Livre
                          </button>
                        )}
                      </div>
                    </td>
                  );
                })}
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
