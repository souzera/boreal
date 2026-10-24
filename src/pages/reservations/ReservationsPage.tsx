import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";

import { DataTable, type ColumnDef, type SortState } from "@/components/data-table/DataTable";
import { FormField } from "@/components/FormField";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  listReservations,
  listReservationsNeedingReview,
  listRooms,
  type Reservation,
  type ReservationStatus,
} from "@/lib/api";
import { formatDate, formatMoney } from "@/lib/format";
import { statusIcon, statusLabel } from "@/lib/reservation";
import { useSessionStore } from "@/stores/session";

const PAGE_SIZE = 20;
const statuses = Object.keys(statusLabel) as ReservationStatus[];

const reasonText: Record<string, string> = {
  no_show_candidate: "chegada vencida (possível no-show)",
  deposit_overdue: "sinal vencido",
};

export default function ReservationsPage() {
  const token = useSessionStore((s) => s.token) ?? "";
  const navigate = useNavigate();
  const [text, setText] = useState("");
  const [status, setStatus] = useState<ReservationStatus | "">("");
  const [roomId, setRoomId] = useState<number | undefined>();
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [page, setPage] = useState(1);
  const [sort, setSort] = useState<SortState>({ key: "check_in_date", direction: "asc" });

  const query = {
    text: text || undefined,
    status: status ? [status] : undefined,
    room_id: roomId,
    from: from || undefined,
    to: to || undefined,
    page,
    page_size: PAGE_SIZE,
    sort: sort.key,
    direction: sort.direction,
  };
  const { data } = useQuery({ queryKey: ["reservations", query], queryFn: () => listReservations(token, query) });
  const { data: rooms } = useQuery({
    queryKey: ["rooms", "options"],
    queryFn: () => listRooms(token, { active: true, page: 1, page_size: 100 }),
  });
  const { data: review = [] } = useQuery({
    queryKey: ["reservations-review"],
    queryFn: () => listReservationsNeedingReview(token),
  });

  const columns: ColumnDef<Reservation>[] = [
    { key: "code", header: "Código", sortable: true, render: (r) => <strong>{r.code}</strong> },
    { key: "guest_name", header: "Hóspede", sortable: true, render: (r) => r.guest_name },
    { key: "room_code", header: "Quarto", sortable: true, render: (r) => r.room_code },
    { key: "check_in_date", header: "Entrada", sortable: true, render: (r) => formatDate(r.check_in_date) },
    { key: "check_out_date", header: "Saída", render: (r) => formatDate(r.check_out_date) },
    { key: "agreed_total_cents", header: "Valor", sortable: true, render: (r) => formatMoney(r.agreed_total_cents) },
    {
      key: "status",
      header: "Situação",
      sortable: true,
      render: (r) => (
        <span className="flex flex-wrap items-center gap-2">
          <span>
            <span aria-hidden="true">{statusIcon[r.status]} </span>
            {statusLabel[r.status]}
          </span>
          {r.deposit_overdue && <span className="text-xs font-medium text-destructive">Sinal vencido</span>}
        </span>
      ),
    },
  ];

  return (
    <section>
      <PageHeader
        title="Reservas"
        description="Reservas de quartos, com situação, valores e prazos."
        action={<Button render={<Link to="/reservas/nova" />}>Nova reserva</Button>}
      />
      {review.length > 0 && (
        <section
          role="region"
          aria-label="Reservas para revisar"
          className="mb-4 rounded-lg border border-destructive/40 bg-destructive/5 p-3 text-sm"
        >
          <p className="font-medium">
            {review.length === 1 ? "1 reserva precisa de uma decisão" : `${review.length} reservas precisam de uma decisão`}
          </p>
          <ul className="mt-1 grid gap-1">
            {review.map(({ reservation, reasons }) => (
              <li key={reservation.id}>
                <Link to={`/reservas/${reservation.id}`} className="underline">
                  {reservation.code} — {reservation.guest_name}
                </Link>
                : {reasons.map((r) => reasonText[r]).join("; ")}
              </li>
            ))}
          </ul>
        </section>
      )}
      <div className="mb-4 flex flex-wrap items-end gap-4">
        <div className="w-56">
          <FormField id="res-search" label="Buscar reserva">
            <Input
              id="res-search"
              placeholder="Código ou nome do hóspede"
              value={text}
              onChange={(e) => {
                setText(e.target.value);
                setPage(1);
              }}
            />
          </FormField>
        </div>
        <FormField id="res-status" label="Situação">
          <select
            id="res-status"
            className="h-8 rounded-lg border border-input bg-transparent px-2 text-sm"
            value={status}
            onChange={(e) => {
              setStatus(e.target.value as ReservationStatus | "");
              setPage(1);
            }}
          >
            <option value="">Todas</option>
            {statuses.map((s) => (
              <option key={s} value={s}>
                {statusLabel[s]}
              </option>
            ))}
          </select>
        </FormField>
        <FormField id="res-room" label="Quarto">
          <select
            id="res-room"
            className="h-8 rounded-lg border border-input bg-transparent px-2 text-sm"
            value={roomId ?? ""}
            onChange={(e) => {
              setRoomId(e.target.value ? Number(e.target.value) : undefined);
              setPage(1);
            }}
          >
            <option value="">Todos</option>
            {rooms?.items.map((r) => (
              <option key={r.id} value={r.id}>
                {r.code}
              </option>
            ))}
          </select>
        </FormField>
        <FormField id="res-from" label="De">
          <Input
            id="res-from"
            type="date"
            value={from}
            onChange={(e) => {
              setFrom(e.target.value);
              setPage(1);
            }}
          />
        </FormField>
        <FormField id="res-to" label="Até">
          <Input
            id="res-to"
            type="date"
            value={to}
            onChange={(e) => {
              setTo(e.target.value);
              setPage(1);
            }}
          />
        </FormField>
      </div>
      <DataTable
        columns={columns}
        rows={data?.items ?? []}
        getRowKey={(r) => r.id}
        total={data?.total ?? 0}
        page={page}
        pageSize={PAGE_SIZE}
        sort={sort}
        emptyMessage="Nenhuma reserva encontrada."
        onPageChange={setPage}
        onSortChange={(next) => {
          setSort(next);
          setPage(1);
        }}
        onRowClick={(r) => navigate(`/reservas/${r.id}`)}
      />
    </section>
  );
}
