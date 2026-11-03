import { useEffect, useRef } from "react";
import { useSearchParams } from "react-router-dom";

/**
 * Executa uma ação pedida por `?abrir=` (por exemplo, abrir o diálogo de check-in) uma única vez e
 * remove o parâmetro do endereço, para que Voltar e recarregar não a repitam.
 *
 * `ready` indica que a página já tem os dados para decidir. `onOpen` devolve `false` quando a ação não
 * cabe mais; nesse caso o parâmetro também é removido e a página deve avisar a pessoa.
 * Valores fora de `allowed` são ignorados e limpos.
 */
export function useOpenFromQuery<T extends string>(allowed: readonly T[], ready: boolean, onOpen: (action: T) => boolean): void {
  const [params, setParams] = useSearchParams();
  const handled = useRef<string | null>(null);
  const requested = params.get("abrir");

  useEffect(() => {
    // Parâmetro limpo: um próximo pedido igual (mesma página, outro registro) volta a valer.
    if (!requested) handled.current = null;
    if (!requested || !ready || handled.current === requested) return;
    handled.current = requested;
    if ((allowed as readonly string[]).includes(requested)) onOpen(requested as T);
    const next = new URLSearchParams(params);
    next.delete("abrir");
    setParams(next, { replace: true });
    // `allowed` e `onOpen` mudam a cada render; só o pedido e a prontidão disparam a ação.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [requested, ready]);
}
