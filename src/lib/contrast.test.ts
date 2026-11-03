import { contrastRatio } from "./contrast";

describe("contrastRatio", () => {
  it("preto sobre branco é 21:1", () => {
    expect(contrastRatio("#000000", "#FFFFFF")).toBeCloseTo(21, 5);
  });

  it("cinza médio sobre branco fica perto de 4,48:1", () => {
    expect(contrastRatio("#777777", "#FFFFFF")).toBeCloseTo(4.48, 2);
  });

  it("é simétrico e aceita minúsculas e três dígitos", () => {
    expect(contrastRatio("#ffffff", "#000000")).toBeCloseTo(contrastRatio("#000000", "#ffffff"), 10);
    expect(contrastRatio("#fff", "#000")).toBeCloseTo(21, 5);
  });

  it("cores iguais têm razão 1", () => {
    expect(contrastRatio("#0F766E", "#0F766E")).toBeCloseTo(1, 10);
  });
});
