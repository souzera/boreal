/** `1,5 GB`, `340 MB`, `12 KB`. */
export function formatBytes(bytes: number): string {
  const KB = 1024;
  if (bytes >= KB ** 3) return `${(bytes / KB ** 3).toFixed(1).replace(".", ",")} GB`;
  if (bytes >= KB ** 2) return `${Math.round(bytes / KB ** 2)} MB`;
  return `${Math.max(1, Math.round(bytes / KB))} KB`;
}

export const CONFIRMATION_WORD = "RESTAURAR";

/** Chave de recuperação em grupos separados por hífen, para mostrar e imprimir. */
export const keyGroups = (text: string): string[] => text.split("-").filter(Boolean);
