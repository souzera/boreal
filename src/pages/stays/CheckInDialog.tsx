import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { usePermissions } from "@/hooks/usePermissions";
import { checkInReservation, listRooms, type ApiError, type Reservation } from "@/lib/api";
import { formatMoney } from "@/lib/format";
import { useGuardedClose } from "@/hooks/useGuardedClose";
import { useSessionStore } from "@/stores/session";
import { hasBlankName, OccupantsEditor, toOccupantInputs, type Companion } from "./OccupantsEditor";

interface Props {
  open: boolean;
  reservation: Reservation;
  onClose: () => void;
  /** Recebe o id da hospedagem criada. */
  onDone: (stayId: number) => void;
  onConflict?: () => void;
}

/** O horário previsto de entrada ainda não chegou? */
function isEarly(reservation: Reservation): boolean {
  return new Date() < new Date(`${reservation.check_in_date}T${reservation.expected_check_in_time}:00`);
}

export default function CheckInDialog({ open, reservation, onClose, onDone, onConflict }: Props) {
  const token = useSessionStore((s) => s.token) ?? "";
  const { can } = usePermissions();
  const [companions, setCompanions] = useState<Companion[]>([]);
  const [exceed, setExceed] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    setCompanions([]);
    setExceed(false);
    setError(null);
  }, [open]);

  const { data: rooms } = useQuery({
    queryKey: ["rooms", "options"],
    queryFn: () => listRooms(token, { active: true, page: 1, page_size: 100 }),
    enabled: open,
  });
  const { requestClose, confirm } = useGuardedClose(companions.length > 0, onClose);

  const room = rooms?.items.find((r) => r.id === reservation.room_id);
  const people = companions.length + 1;
  const overCapacity = room !== undefined && people > room.max_capacity;
  const early = isEarly(reservation);
  const needsEarlyPermission = early && !can("early_check_in");
  const needsExceedPermission = overCapacity && !can("exceed_capacity");
  const mustConfirmExceed = overCapacity && can("exceed_capacity") && !exceed;
  const blocked = needsEarlyPermission || needsExceedPermission || mustConfirmExceed;
  const openingBalance = reservation.agreed_total_cents - reservation.deposit_paid_cents;

  const submit = async () => {
    setError(null);
    if (hasBlankName(companions)) {
      setError("Informe o nome de todos os ocupantes.");
      return;
    }
    setBusy(true);
    try {
      const stayId = await checkInReservation(token, {
        reservation_id: reservation.id,
        expected_version: reservation.version,
        occupants: toOccupantInputs({ id: reservation.guest_id, name: reservation.guest_name }, companions),
        exceed_capacity: overCapacity && exceed,
      });
      onDone(stayId);
    } catch (e) {
      const apiError = e as ApiError;
      setError(apiError.message);
      if (apiError.code === "conflict") onConflict?.();
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <Dialog open={open} onOpenChange={(next) => !next && requestClose()}>
        <DialogContent showCloseButton={false}>
          <DialogHeader>
            <DialogTitle>{`Check-in da reserva ${reservation.code}`}</DialogTitle>
            <DialogDescription>
              {`Quarto ${reservation.room_code} · ${reservation.guest_name}. Confira os ocupantes antes de confirmar.`}
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-4">
            <OccupantsEditor holderName={reservation.guest_name} companions={companions} onChange={setCompanions} />
            {early && (
              <p role="alert" className="rounded-lg border border-destructive/40 bg-destructive/5 p-2 text-sm">
                {"⚠ Check-in antes do horário previsto — exige autorização."}
                {needsEarlyPermission && " Peça a quem pode autorizar o check-in antecipado."}
              </p>
            )}
            {overCapacity && room && (
              <div role="alert" className="grid gap-2 rounded-lg border border-destructive/40 bg-destructive/5 p-2 text-sm">
                <p>{`⚠ O quarto ${room.code} comporta ${room.max_capacity} pessoa(s).`}</p>
                {can("exceed_capacity") ? (
                  <label className="flex items-center gap-2">
                    <input type="checkbox" checked={exceed} onChange={(e) => setExceed(e.target.checked)} />
                    Autorizar exceder a capacidade
                  </label>
                ) : (
                  <p>Reduza os ocupantes ou peça a quem pode autorizar o excesso de capacidade.</p>
                )}
              </div>
            )}
            <p className="text-sm font-medium">{`Saldo inicial: ${formatMoney(openingBalance)}`}</p>
            {error && (
              <p role="alert" className="text-sm text-destructive">
                {error}
              </p>
            )}
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={requestClose}>
              Voltar
            </Button>
            <Button type="button" disabled={busy || blocked} onClick={submit}>
              Confirmar check-in
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      {confirm}
    </>
  );
}
