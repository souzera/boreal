import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { FormField, RequiredLegend } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Textarea } from "@/components/ui/textarea";
import { useGuardedClose } from "@/hooks/useGuardedClose";
import {
  applyReservationChange,
  listRooms,
  previewReservationChange,
  type ApiError,
  type ChangePreview,
  type ChangeRequest,
  type Reservation,
} from "@/lib/api";
import { formatDate, formatMoney } from "@/lib/format";
import { conflictText } from "@/lib/reservation";
import { useSessionStore } from "@/stores/session";

interface Props {
  open: boolean;
  reservation: Reservation;
  onClose: () => void;
  onApplied: (message: string) => void;
  onConflict: () => void;
}

function signed(cents: number): string {
  return cents < 0 ? `− ${formatMoney(-cents)}` : `+ ${formatMoney(cents)}`;
}

/** Alteração de datas ou quarto: mostra preço anterior, novo e diferença antes de gravar (spec §4.2). */
export default function ChangeDialog({ open, reservation, onClose, onApplied, onConflict }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const { data: rooms } = useQuery({
    queryKey: ["rooms", "options"],
    queryFn: () => listRooms(token, { active: true, page: 1, page_size: 100 }),
    enabled: open,
  });
  const [roomId, setRoomId] = useState(reservation.room_id);
  const [checkIn, setCheckIn] = useState(reservation.check_in_date);
  const [checkOut, setCheckOut] = useState(reservation.check_out_date);
  const [justification, setJustification] = useState("");
  const [preview, setPreview] = useState<ChangePreview | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    setRoomId(reservation.room_id);
    setCheckIn(reservation.check_in_date);
    setCheckOut(reservation.check_out_date);
    setJustification("");
    setPreview(null);
    setMessage(null);
  }, [open, reservation.room_id, reservation.check_in_date, reservation.check_out_date]);

  const request: ChangeRequest = {
    room_id: roomId !== reservation.room_id ? roomId : null,
    check_in_date: checkIn !== reservation.check_in_date ? checkIn : null,
    check_out_date: checkOut !== reservation.check_out_date ? checkOut : null,
  };
  const dirty =
    roomId !== reservation.room_id ||
    checkIn !== reservation.check_in_date ||
    checkOut !== reservation.check_out_date ||
    justification !== "";
  const { requestClose, confirm } = useGuardedClose(dirty, onClose);

  const edit = (apply: () => void) => {
    apply();
    setPreview(null); // qualquer mudança exige uma nova prévia
    setMessage(null);
  };

  const loadPreview = async () => {
    setBusy(true);
    setMessage(null);
    try {
      setPreview(await previewReservationChange(token, reservation.id, request));
    } catch (e) {
      setPreview(null);
      setMessage((e as ApiError).message);
    } finally {
      setBusy(false);
    }
  };

  const apply = async () => {
    if (!preview) return;
    setBusy(true);
    setMessage(null);
    try {
      await applyReservationChange(token, reservation.id, request, reservation.version, justification.trim(), preview.fingerprint);
      onApplied("Reserva alterada.");
    } catch (e) {
      const error = e as ApiError;
      setMessage(error.message);
      if (error.code === "stale_preview") {
        try {
          setPreview(await previewReservationChange(token, reservation.id, request));
        } catch {
          setPreview(null);
        }
      } else if (error.code === "conflict") {
        onConflict();
      }
    } finally {
      setBusy(false);
    }
  };

  const canConfirm = !!preview?.feasible && justification.trim() !== "" && !busy;

  return (
    <>
      <Dialog open={open} onOpenChange={(next) => !next && requestClose()}>
        <DialogContent showCloseButton={false} className="max-h-[90vh] overflow-y-auto sm:max-w-xl">
          <DialogHeader>
            <DialogTitle>Alterar datas ou quarto</DialogTitle>
            <DialogDescription>
              As noites que continuam mantêm o preço combinado; as noites novas usam a tabela atual. Nada é gravado antes da confirmação.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4">
            <FormField id="chg-room" label="Novo quarto">
              <select
                id="chg-room"
                className="h-8 rounded-lg border border-input bg-transparent px-2 text-sm"
                value={roomId}
                onChange={(e) => edit(() => setRoomId(Number(e.target.value)))}
              >
                {rooms?.items.map((r) => (
                  <option key={r.id} value={r.id}>
                    {`${r.code} · ${r.category_name}`}
                  </option>
                ))}
                {!rooms && <option value={reservation.room_id}>{reservation.room_code}</option>}
              </select>
            </FormField>
            <div className="grid gap-4 sm:grid-cols-2">
              <FormField id="chg-in" label="Nova entrada">
                <Input id="chg-in" type="date" value={checkIn} onChange={(e) => edit(() => setCheckIn(e.target.value))} />
              </FormField>
              <FormField id="chg-out" label="Nova saída">
                <Input id="chg-out" type="date" value={checkOut} onChange={(e) => edit(() => setCheckOut(e.target.value))} />
              </FormField>
            </div>
            <Button type="button" variant="outline" className="w-fit" disabled={busy} onClick={loadPreview}>
              Ver prévia
            </Button>

            {preview && (
              <div className="grid gap-2 rounded-lg border p-3 text-sm">
                <p>{`Preço anterior: ${formatMoney(preview.price_before_cents)}`}</p>
                <p>{`Preço novo: ${formatMoney(preview.price_after_cents)}`}</p>
                <p className="font-medium">{`Diferença: ${signed(preview.difference_cents)}`}</p>
                <p className="text-muted-foreground">{`Novo sinal esperado: ${formatMoney(preview.new_deposit_expected_cents)}`}</p>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Noite</TableHead>
                      <TableHead>Diária</TableHead>
                      <TableHead>Preço</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {preview.nights.map((n) => (
                      <TableRow key={n.date}>
                        <TableCell>{formatDate(n.date)}</TableCell>
                        <TableCell>{formatMoney(n.amount_cents)}</TableCell>
                        <TableCell>{n.kept ? "Mantém o preço combinado" : "Preço da tabela atual"}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
                {preview.conflicts.length > 0 && (
                  <div role="alert" className="text-destructive">
                    <p>O quarto não está disponível. Reservas que bloqueiam:</p>
                    <ul className="list-disc pl-5">
                      {preview.conflicts.map((c) => (
                        <li key={c.reservation_id}>{`${conflictText(c)} · ${formatDate(c.check_in_date)} a ${formatDate(c.check_out_date)}`}</li>
                      ))}
                    </ul>
                  </div>
                )}
                {preview.missing_dates.length > 0 && (
                  <p role="alert" className="text-destructive">
                    {`Faltam tarifas para: ${preview.missing_dates.map(formatDate).join(", ")}.`}
                  </p>
                )}
                {preview.problems.map((p) => (
                  <p key={p} role="alert" className="text-destructive">
                    {p}
                  </p>
                ))}
              </div>
            )}

            <FormField id="chg-why" label="Justificativa" required>
              <Textarea id="chg-why" value={justification} onChange={(e) => setJustification(e.target.value)} />
            </FormField>
            {message && (
              <p role="alert" className="text-sm text-destructive">
                {message}
              </p>
            )}
            <RequiredLegend />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={requestClose}>
              Cancelar
            </Button>
            <Button disabled={!canConfirm} onClick={apply}>
              Confirmar alteração
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      {confirm}
    </>
  );
}
