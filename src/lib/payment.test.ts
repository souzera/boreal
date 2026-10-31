import { checkSplit, describeComponents, emptyRow, methodLabel, sumRows } from "./payment";

describe("describeComponents", () => {
  it("lista cada forma com o valor, na linguagem da pousada", () => {
    expect(
      describeComponents([
        { method: "pix", amount_cents: 6000 },
        { method: "cash", amount_cents: 4000 },
      ]),
    ).toBe("Pix R$ 60,00 + Dinheiro R$ 40,00");
    expect(describeComponents([{ method: "unspecified", amount_cents: 12000 }])).toBe("Sem modalidade R$ 120,00");
  });

  it("débito e crédito têm rótulos próprios", () => {
    expect(methodLabel.debit).toBe("Cartão de débito");
    expect(methodLabel.credit).toBe("Cartão de crédito");
  });
});

describe("checkSplit", () => {
  it("com uma forma só, o valor é o total informado", () => {
    expect(checkSplit(10000, [{ method: "pix", amount: "" }])).toEqual({
      components: [{ method: "pix", amount_cents: 10000 }],
      sum: 10000,
      remaining: 0,
      error: null,
    });
  });

  it("exige a forma de pagamento e um total válido", () => {
    expect(checkSplit(10000, [emptyRow()]).error).toBe("Escolha a forma de pagamento.");
    expect(checkSplit(null, [{ method: "pix", amount: "" }]).error).toBe("Informe um valor maior que zero, por exemplo R$ 100,00.");
    expect(checkSplit(0, [{ method: "pix", amount: "" }]).error).toBe("Informe um valor maior que zero, por exemplo R$ 100,00.");
  });

  it("dividido: soma os valores e diz quanto falta ou sobra", () => {
    const rows = [
      { method: "pix" as const, amount: "60,00" },
      { method: "cash" as const, amount: "30,00" },
    ];
    expect(checkSplit(10000, rows)).toMatchObject({
      components: null,
      sum: 9000,
      remaining: 1000,
      error: "A soma das formas de pagamento precisa ser R$ 100,00.",
    });
    expect(checkSplit(8000, rows)).toMatchObject({ components: null, sum: 9000, remaining: -1000 });
    expect(checkSplit(9000, rows)).toMatchObject({ remaining: 0, error: null });
    expect(sumRows(rows)).toBe(9000);
  });

  it("dividido: recusa forma repetida, valor inválido e forma em branco", () => {
    const pix = (amount: string) => ({ method: "pix" as const, amount });
    expect(checkSplit(10000, [pix("60,00"), pix("40,00")]).error).toBe("Cada forma de pagamento só pode aparecer uma vez.");
    expect(checkSplit(10000, [pix("60,00"), { method: "cash", amount: "abc" }]).error).toBe("Informe o valor de cada forma de pagamento.");
    expect(checkSplit(10000, [pix("60,00"), { method: "", amount: "40,00" }]).error).toBe("Escolha a forma de pagamento.");
  });
});
