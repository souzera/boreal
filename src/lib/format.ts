/** Formata centavos inteiros como "R$ 1.200,00". */
export function formatMoney(cents: number): string {
  const sign = cents < 0 ? "-" : "";
  const abs = Math.abs(Math.trunc(cents));
  const reais = Math.floor(abs / 100)
    .toString()
    .replace(/\B(?=(\d{3})+(?!\d))/g, ".");
  const centavos = (abs % 100).toString().padStart(2, "0");
  return `${sign}R$ ${reais},${centavos}`;
}

const MONEY_PATTERN = /^(\d{1,3}(\.\d{3})+|\d+)(,\d{1,2})?$/;

/** Converte texto como "R$ 1.200,50" em centavos inteiros; `null` se inválido. */
export function parseMoney(input: string): number | null {
  const text = input.replace(/^R\$\s*/, "").trim();
  if (!MONEY_PATTERN.test(text)) return null;
  const [intPart, decPart = ""] = text.replace(/\./g, "").split(",");
  return Number(intPart) * 100 + Number(decPart.padEnd(2, "0"));
}

/** Converte data de calendário ISO (YYYY-MM-DD) em DD/MM/AAAA, sem conversão de fuso. */
export function formatDate(iso: string): string {
  const [year, month, day] = iso.split("-");
  return `${day}/${month}/${year}`;
}

/** Exibe um instante UTC como HH:MM no fuso informado (padrão: o do computador). */
export function formatTime(utcIso: string, timeZone?: string): string {
  return new Intl.DateTimeFormat("pt-BR", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
    timeZone,
  }).format(new Date(utcIso));
}
