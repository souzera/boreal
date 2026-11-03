import { cn } from "@/lib/utils";

export type IconSize = "xs" | "sm" | "md" | "lg";

/** Escala do brief: ações compactas 14, botões e tabelas 16, navegação 20, indicadores 22. */
export const iconSizePx: Record<IconSize, number> = { xs: 14, sm: 16, md: 20, lg: 22 };

interface Props {
  /** Nome do símbolo Material Symbols (por exemplo `warning`, `bed`). */
  name: string;
  size?: IconSize;
  className?: string;
}

/**
 * Única fonte de ícones da interface (Material Symbols Rounded, preenchido, empacotado no app).
 * É decorativo: o significado sempre vem de um rótulo ao lado, nunca só do ícone.
 */
export default function Icon({ name, size = "sm", className }: Props) {
  return (
    <span
      aria-hidden="true"
      data-icon={name}
      className={cn("material-symbols-rounded inline-block h-[1em] w-[1em] shrink-0 select-none overflow-hidden leading-none", className)}
      style={{ fontSize: iconSizePx[size] }}
    >
      {name}
    </span>
  );
}
