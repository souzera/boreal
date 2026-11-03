import { BlockItems } from "@/components/BlockItems";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { Link, useNavigate } from "react-router-dom";

import { FormField, RequiredLegend } from "@/components/FormField";
import { PageHeader } from "@/components/PageHeader";
import { UnsavedChangesGuard } from "@/components/UnsavedChangesGuard";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { usePermissions } from "@/hooks/usePermissions";
import { checkRoomAvailability, listGuests, listRooms, quoteStay, walkIn, type ApiError, type Guest } from "@/lib/api";
import { formatDate, formatMoney, formatPhone, toLocalIsoDate } from "@/lib/format";
import { conflictText, ruleLabel, statusLabel } from "@/lib/reservation";
import { useSessionStore } from "@/stores/session";
import { hasBlankName, OccupantsEditor, toOccupantInputs, type Companion } from "./OccupantsEditor";

type Errors = Partial<Record<"guest" | "room" | "checkOut" | "occupants", string>>;

/** Hospedagem sem reserva: entra hoje, agora, depois de conferir disponibilidade, preço e capacidade. */
export default function WalkInPage() {
  const token = useSessionStore((s) => s.token) ?? "";
  const navigate = useNavigate();
  const { can } = usePermissions();
  const today = toLocalIsoDate(new Date());

  const [guestText, setGuestText] = useState("");
  const [guest, setGuest] = useState<Guest | null>(null);
  const [roomId, setRoomId] = useState<number | null>(null);
  const [checkOut, setCheckOut] = useState("");
  const [companions, setCompanions] = useState<Companion[]>([]);
  const [exceed, setExceed] = useState(false);
  const [errors, setErrors] = useState<Errors>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  // Sair só depois de a tela ser renderizada sem o aviso de alterações não salvas.
  const [leaveTo, setLeaveTo] = useState<{ path: string; notice: string } | null>(null);
  useEffect(() => {
    if (leaveTo) navigate(leaveTo.path, { state: { notice: leaveTo.notice } });
  }, [leaveTo, navigate]);

  const dirty = leaveTo === null && (guest !== null || guestText !== "" || checkOut !== "" || companions.length > 0 || roomId !== null);

  const { data: found } = useQuery({
    queryKey: ["guest-search", guestText],
    queryFn: () => listGuests(token, { text: guestText, active: true, page: 1, page_size: 8 }),
    enabled: guestText.trim().length >= 2 && guest === null,
  });
  const { data: rooms } = useQuery({
    queryKey: ["rooms", "options"],
    queryFn: () => listRooms(token, { active: true, page: 1, page_size: 100 }),
  });

  const room = rooms?.items.find((r) => r.id === roomId) ?? null;
  const live = room !== null && checkOut !== "" && checkOut > today;
  const { data: availability } = useQuery({
    queryKey: ["availability", roomId, today, checkOut],
    queryFn: () => checkRoomAvailability(token, roomId as number, today, checkOut),
    enabled: live,
  });
  const { data: quote } = useQuery({
    queryKey: ["quote", room?.category_id, today, checkOut],
    queryFn: () => quoteStay(token, room?.category_id as number, today, checkOut),
    enabled: live,
  });

  const people = companions.length + 1;
  const overCapacity = room !== null && people > room.max_capacity;
  const mayExceed = can("exceed_capacity");
  const blocked =
    (availability && (!availability.available || !!availability.not_ready)) ||
    (quote && quote.missing_dates.length > 0) ||
    (overCapacity && (!mayExceed || !exceed));

  const validate = (): Errors => {
    const next: Errors = {};
    if (!guest) next.guest = "Escolha o hóspede.";
    if (!room) next.room = "Escolha o quarto.";
    if (!checkOut) next.checkOut = "Informe a data de saída.";
    else if (checkOut <= today) next.checkOut = "A saída deve ser depois de hoje.";
    if (hasBlankName(companions)) next.occupants = "Informe o nome de todos os ocupantes.";
    return next;
  };

  const submit = async (e: { preventDefault: () => void }) => {
    e.preventDefault();
    setFormError(null);
    const next = validate();
    setErrors(next);
    if (Object.keys(next).length > 0 || !guest || !room) return;
    setSaving(true);
    try {
      const stayId = await walkIn(token, {
        room_id: room.id,
        guest_id: guest.id,
        check_out_date: checkOut,
        expected_check_out_time: null,
        occupants: toOccupantInputs({ id: guest.id, name: guest.full_name }, companions),
        exceed_capacity: overCapacity && exceed,
      });
      setLeaveTo({ path: `/hospedagens/${stayId}`, notice: "Hospedagem iniciada e salva neste computador." });
    } catch (err) {
      setFormError((err as ApiError).message);
    } finally {
      setSaving(false);
    }
  };

  return (
    <section className="max-w-3xl">
      <PageHeader
        title="Hospedagem sem reserva"
        description="Para quem chega sem reserva: confere disponibilidade e preço antes de iniciar a hospedagem."
        action={
          <Button variant="outline" render={<Link to="/hospedagens" />}>
            Voltar às hospedagens
          </Button>
        }
      />
      <form onSubmit={submit} noValidate className="grid gap-4 sm:grid-cols-2">
        <div className="grid gap-2 sm:col-span-2">
          <FormField id="wi-guest" label="Hóspede" required error={errors.guest}>
            {guest ? (
              <div className="flex items-center justify-between gap-2 rounded-lg border p-2 text-sm">
                <span>
                  <strong>{guest.full_name}</strong> · {guest.code} · {formatPhone(guest.phone)}
                </span>
                <Button type="button" size="sm" variant="outline" onClick={() => setGuest(null)}>
                  Trocar hóspede
                </Button>
              </div>
            ) : (
              <Input
                id="wi-guest"
                placeholder="Digite nome, telefone, CPF ou código"
                value={guestText}
                onChange={(e) => setGuestText(e.target.value)}
              />
            )}
          </FormField>
          {!guest && found && found.items.length > 0 && (
            <ul className="grid gap-1">
              {found.items.map((g) => (
                <li key={g.id}>
                  <Button type="button" variant="outline" size="sm" className="w-full justify-start" onClick={() => setGuest(g)}>
                    {g.full_name} · {g.code} · {formatPhone(g.phone)}
                  </Button>
                </li>
              ))}
            </ul>
          )}
          {!guest && (
            <p className="text-sm text-muted-foreground">
              Não encontrou? <Link to="/hospedes/novo" className="underline">Cadastrar novo hóspede</Link>
            </p>
          )}
        </div>
        <FormField id="wi-room" label="Quarto" required error={errors.room}>
          <select
            id="wi-room"
            className="h-8 rounded-lg border border-input bg-transparent px-2 text-sm"
            value={roomId ?? ""}
            onChange={(e) => setRoomId(e.target.value ? Number(e.target.value) : null)}
          >
            <option value="">Escolha…</option>
            {rooms?.items.map((r) => (
              <option key={r.id} value={r.id}>
                {`${r.code} · ${r.category_name} · até ${r.max_capacity} pessoas`}
              </option>
            ))}
          </select>
        </FormField>
        <FormField id="wi-out" label="Saída" required error={errors.checkOut}>
          <Input id="wi-out" type="date" value={checkOut} onChange={(e) => setCheckOut(e.target.value)} />
        </FormField>

        <div className="grid gap-1 sm:col-span-2">
          <OccupantsEditor holderName={guest?.full_name ?? null} companions={companions} onChange={setCompanions} />
          {errors.occupants && (
            <p role="alert" className="text-sm text-destructive">
              {errors.occupants}
            </p>
          )}
        </div>

        {overCapacity && room && (
          <div className="grid gap-2 sm:col-span-2" role="alert">
            <p className="text-sm text-destructive">
              {`O quarto ${room.code} comporta ${room.max_capacity} pessoa(s).`}
              {!mayExceed && " Reduza os ocupantes ou peça a quem pode autorizar o excesso."}
            </p>
            {mayExceed && (
              <label className="flex items-center gap-2 text-sm">
                <input type="checkbox" checked={exceed} onChange={(e) => setExceed(e.target.checked)} />
                Autorizar exceder a capacidade
              </label>
            )}
          </div>
        )}

        {live && availability && (
          <div className="sm:col-span-2" role="status">
            {availability.not_ready && <p className="text-sm text-destructive">{availability.not_ready}</p>}
            {availability.available ? (
              !availability.not_ready && <p className="text-sm">Quarto disponível agora.</p>
            ) : (
              <div className="text-sm text-destructive">
                <p>O quarto não está disponível. O que impede o período:</p>
                <ul className="list-disc pl-5">
                  {availability.conflicts.map((c) => (
                    <li key={c.reservation_id}>
                      {conflictText(c)} · {formatDate(c.check_in_date)} a {formatDate(c.check_out_date)} · {statusLabel[c.status]}
                    </li>
                  ))}
                  <BlockItems blocks={availability.blocks} />
                </ul>
              </div>
            )}
          </div>
        )}

        {live && quote && (
          <div className="grid gap-2 sm:col-span-2">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Noite</TableHead>
                  <TableHead>Diária</TableHead>
                  <TableHead>Regra aplicada</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {quote.nights.map((n) => (
                  <TableRow key={n.date}>
                    <TableCell>{formatDate(n.date)}</TableCell>
                    <TableCell>{n.amount_cents === 0 ? "—" : formatMoney(n.amount_cents)}</TableCell>
                    <TableCell>{ruleLabel[n.rule]}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
            {quote.missing_dates.length > 0 ? (
              <p role="alert" className="text-sm text-destructive">
                {`Faltam tarifas para: ${quote.missing_dates.map(formatDate).join(", ")}. Cadastre as tarifas antes de hospedar.`}
              </p>
            ) : (
              <p className="text-sm">
                Total: <strong>{formatMoney(quote.total_cents)}</strong>
              </p>
            )}
          </div>
        )}

        <div className="grid gap-3 sm:col-span-2">
          {formError && (
            <p role="alert" className="text-sm text-destructive">
              {formError}
            </p>
          )}
          <RequiredLegend />
          <Button type="submit" className="w-fit" disabled={saving || !!blocked}>
            Confirmar check-in
          </Button>
        </div>
      </form>
      <UnsavedChangesGuard isDirty={dirty} />
    </section>
  );
}
