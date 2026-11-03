import { alertCountLabel, kindOptions, severityIcon, severityLabel } from "./alerts";

describe("alertas na interface", () => {
  it("cada prioridade tem ícone e rótulo", () => {
    expect(severityIcon).toEqual({ critical: "error", high: "warning", normal: "info" });
    expect(severityLabel).toEqual({ critical: "Crítico", high: "Alto", normal: "Normal" });
  });

  it("a lista de tipos tem os 14 tipos, sem repetição", () => {
    const values = kindOptions.map((k) => k.value);
    expect(values).toHaveLength(14);
    expect(new Set(values).size).toBe(14);
    expect(kindOptions.find((k) => k.value === "payable_overdue")?.label).toBe("Conta vencida");
  });

  it("descreve a contagem em palavras", () => {
    expect(alertCountLabel({ critical: 0, high: 0, normal: 0, total: 0 })).toBe("Nenhum alerta ativo");
    expect(alertCountLabel({ critical: 0, high: 1, normal: 0, total: 1 })).toBe("1 alerta ativo");
    expect(alertCountLabel({ critical: 1, high: 1, normal: 1, total: 3 })).toBe("3 alertas ativos, 1 crítico");
    expect(alertCountLabel({ critical: 2, high: 0, normal: 0, total: 2 })).toBe("2 alertas ativos, 2 críticos");
  });
});
