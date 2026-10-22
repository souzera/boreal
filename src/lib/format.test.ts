import { formatCpf, formatDate, formatMoney, formatPhone, formatTime, onlyDigits, parseMoney } from "./format";

describe("formatMoney", () => {
  it("formata centavos no padrão brasileiro", () => {
    expect(formatMoney(15000)).toBe("R$ 150,00");
    expect(formatMoney(120000)).toBe("R$ 1.200,00");
    expect(formatMoney(1)).toBe("R$ 0,01");
  });
  it("formata valores negativos", () => {
    expect(formatMoney(-2550)).toBe("-R$ 25,50");
  });
});

describe("parseMoney", () => {
  it("converte texto em centavos inteiros", () => {
    expect(parseMoney("150,75")).toBe(15075);
    expect(parseMoney("R$ 1.200,00")).toBe(120000);
    expect(parseMoney("0,01")).toBe(1);
  });
  it("rejeita entradas inválidas", () => {
    expect(parseMoney("abc")).toBeNull();
    expect(parseMoney("1,999")).toBeNull();
    expect(parseMoney("")).toBeNull();
  });
});

describe("formatDate", () => {
  it("converte ISO em DD/MM/AAAA sem depender de fuso", () => {
    expect(formatDate("2026-03-01")).toBe("01/03/2026");
  });
});

describe("formatTime", () => {
  it("exibe HH:MM no fuso informado", () => {
    expect(formatTime("2026-10-21T15:30:00Z", "America/Sao_Paulo")).toBe("12:30");
  });
});

describe("onlyDigits / formatCpf / formatPhone", () => {
  it("mantém só os dígitos", () => {
    expect(onlyDigits("(11) 99999-0000")).toBe("11999990000");
    expect(onlyDigits("abc")).toBe("");
  });

  it("formata CPF, inclusive parcial", () => {
    expect(formatCpf("12345678909")).toBe("123.456.789-09");
    expect(formatCpf("123456")).toBe("123.456");
    expect(formatCpf("")).toBe("");
  });

  it("formata telefone de 10 e 11 dígitos, inclusive parcial", () => {
    expect(formatPhone("11999990000")).toBe("(11) 99999-0000");
    expect(formatPhone("1133334444")).toBe("(11) 3333-4444");
    expect(formatPhone("119")).toBe("(11) 9");
    expect(formatPhone("")).toBe("");
  });
});
