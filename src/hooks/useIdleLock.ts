import { useEffect } from "react";

const EVENTS = ["mousemove", "mousedown", "keydown", "wheel", "touchstart"] as const;

/** Chama `onIdle` depois de `minutes` sem atividade. O backend segue como autoridade. */
export function useIdleLock(minutes: number, onIdle: () => void, enabled = true) {
  useEffect(() => {
    if (!enabled) return;
    let timer = window.setTimeout(onIdle, minutes * 60_000);
    const reset = () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(onIdle, minutes * 60_000);
    };
    EVENTS.forEach((e) => window.addEventListener(e, reset, { passive: true }));
    return () => {
      window.clearTimeout(timer);
      EVENTS.forEach((e) => window.removeEventListener(e, reset));
    };
  }, [minutes, onIdle, enabled]);
}
