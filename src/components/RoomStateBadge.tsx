import { Link } from "react-router-dom";

import type { RoomStateKey } from "@/lib/api";
import { ROOM_STATE_LABEL, ROOM_STATE_STYLE } from "@/lib/roomState";
import { cn } from "@/lib/utils";

interface Props {
  state: RoomStateKey;
  /** Quando o quarto fica livre (`AAAA-MM-DDTHH:MM`), no estado "limpo, aguardando intervalo". */
  freeAt?: string | null;
  /** Destino do clique (por exemplo, a fila de limpeza). */
  to?: string;
}

/** Estado do quarto em cor, símbolo e rótulo: a cor nunca é o único sinal. */
export function RoomStateBadge({ state, freeAt, to }: Props) {
  const style = ROOM_STATE_STYLE[state];
  const content = (
    <>
      <span aria-hidden="true">{style.icon} </span>
      {ROOM_STATE_LABEL[state]}
      {state === "clean_waiting_interval" && freeAt && <span className="block font-normal">{`livre às ${freeAt.slice(11, 16)}`}</span>}
    </>
  );
  const className = cn("inline-block rounded border px-2 py-0.5 text-xs font-medium", style.className);
  return to ? (
    <Link to={to} className={cn(className, "underline-offset-2 hover:underline")}>
      {content}
    </Link>
  ) : (
    <span className={className}>{content}</span>
  );
}
