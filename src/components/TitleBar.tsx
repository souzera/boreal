import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";

import Icon from "@/components/Icon";
import { LogoMark } from "@/components/Logo";
import { cn } from "@/lib/utils";

/** Altura da barra, em px; também vira a variável CSS `--titlebar-h` que as telas de página inteira descontam. */
export const TITLEBAR_HEIGHT = 36;

const insideTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const control =
  "grid h-full w-11 place-items-center text-foreground outline-none transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring";

/**
 * Barra de título própria: a janela do Windows fica sem moldura e esta barra faz o papel dela
 * (arrastar, duplo clique para maximizar, minimizar, maximizar/restaurar e fechar).
 * Fora do aplicativo (navegador, testes) não aparece, porque lá a janela é do navegador.
 * Fica acima dos diálogos para os controles da janela nunca ficarem inacessíveis.
 */
export default function TitleBar() {
  const active = insideTauri();
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    if (!active) return;
    document.documentElement.style.setProperty("--titlebar-h", `${TITLEBAR_HEIGHT}px`);
    const win = getCurrentWindow();
    let live = true;
    const sync = () => {
      void win.isMaximized().then((value) => live && setMaximized(value));
    };
    sync();
    const unlisten = win.onResized(sync);
    return () => {
      live = false;
      document.documentElement.style.setProperty("--titlebar-h", "0px");
      void unlisten.then((off) => off());
    };
  }, [active]);

  if (!active) return null;
  const win = getCurrentWindow();

  return (
    <div
      role="toolbar"
      aria-label="Janela"
      data-tauri-drag-region
      style={{ height: TITLEBAR_HEIGHT }}
      className="sticky top-0 z-[60] flex select-none items-center justify-between border-b bg-card"
    >
      <div data-tauri-drag-region className="pointer-events-none flex items-center gap-2 pl-3 text-sm font-medium">
        <LogoMark size={18} />
        <span>Boreal PMS</span>
      </div>
      <div className="flex h-full">
        <button type="button" aria-label="Minimizar" className={control} onClick={() => void win.minimize()}>
          <Icon name="remove" size="sm" />
        </button>
        <button
          type="button"
          aria-label={maximized ? "Restaurar" : "Maximizar"}
          className={control}
          onClick={() => void win.toggleMaximize()}
        >
          <Icon name={maximized ? "filter_none" : "crop_square"} size="sm" />
        </button>
        <button
          type="button"
          aria-label="Fechar"
          className={cn(control, "hover:bg-destructive hover:text-white focus-visible:bg-destructive focus-visible:text-white")}
          onClick={() => void win.close()}
        >
          <Icon name="close" size="sm" />
        </button>
      </div>
    </div>
  );
}
