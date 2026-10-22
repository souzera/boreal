/** Valida CPF (dígitos verificadores); aceita máscara. Espelha a validação do backend. */
export function isValidCpf(input: string): boolean {
  const text = input.trim();
  if (/[^\d.\- ]/.test(text)) return false;
  const digits = text.replace(/\D/g, "").split("").map(Number);
  if (digits.length !== 11 || digits.every((d) => d === digits[0])) return false;

  const check = (count: number) => {
    const sum = digits.slice(0, count).reduce((acc, d, i) => acc + d * (count + 1 - i), 0);
    const rest = (sum * 10) % 11;
    return rest === 10 ? 0 : rest;
  };
  return check(9) === digits[9] && check(10) === digits[10];
}
