import type { RoomBlock } from "@/lib/api";
import { blockStatusLabel, periodText } from "@/lib/blocks";

/** Itens de lista com os bloqueios que impedem o período: motivo, período e situação em texto. */
export function BlockItems({ blocks }: { blocks: RoomBlock[] }) {
  return (
    <>
      {blocks.map((b) => (
        <li key={b.id}>
          {`Quarto ${b.room_code} · ${blockStatusLabel[b.status]} · ${periodText(b.start_date, b.end_date)} · ${b.reason}`}
        </li>
      ))}
    </>
  );
}
