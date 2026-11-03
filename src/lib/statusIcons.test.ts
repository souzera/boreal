import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";

import { blockStatusIcon, blockStatusLabel, maintenanceStatusIcon, maintenanceStatusLabel } from "./blocks";
import { statusIcon, statusLabel } from "./reservation";
import { PRIORITY_ICON, PRIORITY_LABEL, ROOM_STATE_LABEL, ROOM_STATE_ORDER, ROOM_STATE_STYLE } from "./roomState";
import { stayStatusIcon, stayStatusLabel } from "./stay";

const GLYPHS = /[●◐◑◔○■✓✔✖⚠▲▽↩⏸]/;
/** Nome de símbolo Material Symbols: minúsculas, dígitos e sublinhado. */
const SYMBOL = /^[a-z][a-z0-9_]*$/;

describe("marcas de estado são ícones nomeados, sempre com rótulo", () => {
  it("cada estado de quarto tem ícone, rótulo e classes do tema", () => {
    for (const state of ROOM_STATE_ORDER) {
      const style = ROOM_STATE_STYLE[state];
      expect(style.icon, state).toMatch(SYMBOL);
      expect(ROOM_STATE_LABEL[state], state).toBeTruthy();
      const token = state.replace(/_/g, "-");
      expect(style.className, state).toContain(`bg-state-${token}-bg`);
      expect(style.className, state).toContain(`text-state-${token}-fg`);
      expect(style.className, state).toContain(`border-state-${token}-border`);
    }
  });

  it("os seis estados de quarto usam ícones diferentes", () => {
    const icons = ROOM_STATE_ORDER.map((s) => ROOM_STATE_STYLE[s].icon);
    expect(new Set(icons).size).toBe(6);
  });

  it("reserva, hospedagem, bloqueio, manutenção e prioridade têm ícone e rótulo", () => {
    const groups: [Record<string, string>, Record<string, string>][] = [
      [statusIcon, statusLabel],
      [stayStatusIcon, stayStatusLabel],
      [blockStatusIcon, blockStatusLabel],
      [maintenanceStatusIcon, maintenanceStatusLabel],
      [PRIORITY_ICON, PRIORITY_LABEL],
    ];
    for (const [icons, labels] of groups) {
      expect(Object.keys(icons).sort()).toEqual(Object.keys(labels).sort());
      for (const [key, name] of Object.entries(icons)) {
        expect(name, key).toMatch(SYMBOL);
        expect(labels[key], key).toBeTruthy();
      }
    }
  });

  it("nenhum símbolo de texto antigo sobra no código (só Icon)", () => {
    const offenders: string[] = [];
    const walk = (dir: string) => {
      for (const entry of readdirSync(dir)) {
        const path = join(dir, entry);
        if (statSync(path).isDirectory()) walk(path);
        else if (/\.(ts|tsx)$/.test(entry) && !/\.test\./.test(entry) && GLYPHS.test(readFileSync(path, "utf-8"))) offenders.push(path);
      }
    };
    walk(resolve(__dirname, ".."));
    expect(offenders).toEqual([]);
  });
});
