import { isValidCpf } from "./cpf";

describe("isValidCpf", () => {
  it("aceita CPF válido, com ou sem máscara", () => {
    expect(isValidCpf("529.982.247-25")).toBe(true);
    expect(isValidCpf("52998224725")).toBe(true);
    expect(isValidCpf("168.995.350-09")).toBe(true);
  });

  it("recusa dígitos repetidos, verificadores errados, tamanho errado e letras", () => {
    for (const bad of ["111.111.111-11", "52998224724", "5299822472", "529982247250", "", "52a98224725"]) {
      expect(isValidCpf(bad)).toBe(false);
    }
  });
});
