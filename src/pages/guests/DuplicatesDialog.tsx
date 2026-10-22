import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { DuplicateMatch, DuplicateReason, Guest } from "@/lib/api";
import { formatPhone } from "@/lib/format";

const reasonLabel: Record<DuplicateReason, string> = {
  name: "nome parecido",
  phone: "mesmo telefone",
  cpf: "mesmo CPF",
};

interface Props {
  matches: DuplicateMatch[] | null;
  onUseExisting: (guest: Guest) => void;
  onCreateAnother: () => void;
  onCancel: () => void;
  busy?: boolean;
}

/** Conferência de possíveis duplicidades antes de criar outro cadastro (spec §3.2). */
export default function DuplicatesDialog({ matches, onUseExisting, onCreateAnother, onCancel, busy }: Props) {
  return (
    <Dialog open={matches !== null} onOpenChange={(next) => !next && onCancel()}>
      <DialogContent showCloseButton={false}>
        <DialogHeader>
          <DialogTitle>Esse hóspede já pode estar cadastrado</DialogTitle>
          <DialogDescription>
            Confira os cadastros abaixo. Se for a mesma pessoa, use o cadastro existente.
          </DialogDescription>
        </DialogHeader>
        <ul className="grid gap-3">
          {matches?.map(({ guest, reasons }) => (
            <li key={guest.id} className="grid gap-1 rounded-lg border p-3 text-sm">
              <div className="flex items-center justify-between gap-2">
                <strong>{guest.full_name}</strong>
                <span className="text-muted-foreground">{guest.code}</span>
              </div>
              <span>{formatPhone(guest.phone)}</span>
              <span className="text-muted-foreground">
                {reasons.map((r) => reasonLabel[r]).join(", ")}
                {guest.active ? "" : " · cadastro inativo"}
              </span>
              <Button size="sm" variant="outline" className="w-fit" onClick={() => onUseExisting(guest)}>
                Usar este cadastro
              </Button>
            </li>
          ))}
        </ul>
        <DialogFooter>
          <Button variant="outline" onClick={onCancel}>
            Voltar e revisar
          </Button>
          <Button disabled={busy} onClick={onCreateAnother}>
            Criar outro cadastro
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
