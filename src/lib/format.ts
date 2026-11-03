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

/** Exibe um instante UTC como "DD/MM/AAAA HH:MM" no fuso informado (padrão: o do computador). */
export function formatInstant(utcIso: string, timeZone?: string): string {
  const parts = new Intl.DateTimeFormat("pt-BR", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
    timeZone,
  }).formatToParts(new Date(utcIso));
  const get = (type: string) => parts.find((p) => p.type === type)?.value ?? "";
  return `${get("day")}/${get("month")}/${get("year")} ${get("hour")}:${get("minute")}`;
}

/** Mantém só os dígitos de um texto. */
export function onlyDigits(input: string): string {
  return input.replace(/\D/g, "");
}

/** Formata CPF ("123.456.789-09"), também enquanto está sendo digitado. */
export function formatCpf(input: string): string {
  const d = onlyDigits(input).slice(0, 11);
  const parts = [d.slice(0, 3), d.slice(3, 6), d.slice(6, 9)].filter(Boolean).join(".");
  return d.length > 9 ? `${parts}-${d.slice(9)}` : parts;
}

/** Formata telefone brasileiro de 10 ou 11 dígitos, também enquanto está sendo digitado. */
export function formatPhone(input: string): string {
  let digits = onlyDigits(input);
  // Código do país (+55) só quando sobram 12 ou 13 dígitos; 55 também é um DDD válido.
  if ((digits.length === 12 || digits.length === 13) && digits.startsWith("55")) digits = digits.slice(2);
  const d = digits.slice(0, 11);
  if (d.length === 0) return "";
  if (d.length <= 2) return `(${d}`;
  const rest = d.slice(2);
  const split = d.length > 10 ? 5 : 4;
  const local = rest.length > split ? `${rest.slice(0, split)}-${rest.slice(split)}` : rest;
  return `(${d.slice(0, 2)}) ${local}`;
}

/** Data de calendário local (`YYYY-MM-DD`) de um instante. */
export function toLocalIsoDate(date: Date): string {
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${date.getFullYear()}-${month}-${day}`;
}
