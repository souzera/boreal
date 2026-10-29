import { formatLocalDateTime, PRIORITY_ICON, PRIORITY_LABEL, ROOM_STATE_LABEL, ROOM_STATE_ORDER, ROOM_STATE_STYLE } from "./roomState";

describe("roomState", () => {
  it("todo estado tem rótulo, cor e símbolo distintos", () => {
    expect(ROOM_STATE_ORDER).toHaveLength(6);
    const labels = new Set(ROOM_STATE_ORDER.map((s) => ROOM_STATE_LABEL[s]));
    const icons = new Set(ROOM_STATE_ORDER.map((s) => ROOM_STATE_STYLE[s].icon));
    const colors = new Set(ROOM_STATE_ORDER.map((s) => ROOM_STATE_STYLE[s].className));
    expect(labels.size).toBe(6);
    expect(icons.size).toBe(6);
    expect(colors.size).toBe(6);
  });

  it("usa os rótulos da spec", () => {
    expect(ROOM_STATE_LABEL.awaiting_cleaning).toBe("Aguardando limpeza");
    expect(ROOM_STATE_LABEL.blocked).toBe("Em manutenção ou bloqueado");
  });

  it("prioridades têm rótulo e símbolo próprios", () => {
    expect(new Set(Object.values(PRIORITY_LABEL)).size).toBe(4);
    expect(new Set(Object.values(PRIORITY_ICON)).size).toBe(4);
  });

  it("formata data e hora locais em DD/MM/AAAA HH:MM", () => {
    expect(formatLocalDateTime("2026-10-23T14:00")).toBe("23/10/2026 14:00");
  });
});
