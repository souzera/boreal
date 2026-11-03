import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { OccupantInput } from "@/lib/api";

export interface Companion {
  name: string;
  document: string;
}

/** Lista de ocupantes: o titular é fixo; os acompanhantes podem ser adicionados e removidos. */
export function OccupantsEditor({
  holderName,
  companions,
  onChange,
}: {
  holderName: string | null;
  companions: Companion[];
  onChange: (next: Companion[]) => void;
}) {
  const update = (index: number, patch: Partial<Companion>) =>
    onChange(companions.map((c, i) => (i === index ? { ...c, ...patch } : c)));

  return (
    <fieldset className="grid gap-2">
      <legend className="text-sm font-medium">Ocupantes</legend>
      <p className="text-sm">{holderName ? `${holderName} (titular)` : "Escolha o hóspede titular."}</p>
      {companions.map((c, i) => (
        <div key={i} className="flex flex-wrap items-end gap-2">
          <Input
            aria-label={`Nome do ocupante ${i + 2}`}
            placeholder="Nome completo"
            className="w-64"
            value={c.name}
            onChange={(e) => update(i, { name: e.target.value })}
          />
          <Input
            aria-label={`Documento do ocupante ${i + 2}`}
            placeholder="Documento (opcional)"
            className="w-48"
            value={c.document}
            onChange={(e) => update(i, { document: e.target.value })}
          />
          <Button type="button" size="sm" variant="outline" onClick={() => onChange(companions.filter((_, j) => j !== i))}>
            {`Remover ocupante ${i + 2}`}
          </Button>
        </div>
      ))}
      <div className="flex items-center gap-3">
        <Button type="button" size="sm" variant="outline" onClick={() => onChange([...companions, { name: "", document: "" }])}>
          Adicionar ocupante
        </Button>
        <span className="text-sm text-muted-foreground">{`${companions.length + 1} ${companions.length === 0 ? "pessoa" : "pessoas"}`}</span>
      </div>
    </fieldset>
  );
}

/** Lista completa enviada ao backend: o titular primeiro, depois os acompanhantes. */
export function toOccupantInputs(holder: { id: number; name: string }, companions: Companion[]): OccupantInput[] {
  return [
    { full_name: holder.name, guest_id: holder.id, document: "" },
    ...companions.map((c) => ({ full_name: c.name, guest_id: null, document: c.document })),
  ];
}

/** `true` quando algum acompanhante está sem nome. */
export const hasBlankName = (companions: Companion[]) => companions.some((c) => c.name.trim() === "");
