import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { contrastRatio } from "@/lib/contrast";

const css = readFileSync(resolve(__dirname, "index.css"), "utf-8");
const html = readFileSync(resolve(__dirname, "../index.html"), "utf-8");

/** Variáveis do bloco `:root` (`--nome: valor;`). */
function rootTokens(): Record<string, string> {
  const block = /:root\s*\{([\s\S]*?)\n\}/.exec(css)?.[1] ?? "";
  const tokens: Record<string, string> = {};
  for (const match of block.matchAll(/--([\w-]+)\s*:\s*([^;]+);/g)) tokens[match[1]] = match[2].trim();
  return tokens;
}

const t = rootTokens();
const STATES = ["occupied", "awaiting-cleaning", "cleaning", "clean-waiting-interval", "available", "blocked"];
const SEMANTIC = ["success", "info", "warning", "danger"];

describe("tema do Boreal (index.css)", () => {
  it("define todos os tokens necessários", () => {
    const required = [
      "background", "foreground", "card", "card-foreground", "popover", "popover-foreground", "primary",
      "primary-foreground", "primary-hover", "primary-subtle", "primary-selected", "secondary", "secondary-foreground",
      "muted", "muted-foreground", "accent", "accent-foreground", "border", "input", "ring", "destructive",
      "sidebar", "sidebar-foreground", "radius", "radius-card", "shadow-card", "shadow-overlay",
      ...SEMANTIC.flatMap((s) => [s, `${s}-subtle`, `${s}-foreground`]),
      ...STATES.flatMap((s) => [`state-${s}-bg`, `state-${s}-fg`, `state-${s}-border`]),
    ];
    for (const name of required) expect(t[name], `--${name}`).toBeTruthy();
  });

  it("a paleta neutra e a identidade seguem o brief", () => {
    expect(t.background.toUpperCase()).toBe("#F4F7FA");
    expect(t.card.toUpperCase()).toBe("#FFFFFF");
    expect(t.muted.toUpperCase()).toBe("#E7EDF3");
    expect(t.border.toUpperCase()).toBe("#CBD5E1");
    expect(t.foreground.toUpperCase()).toBe("#172033");
    // O brief sugere #64748B, que dá 4,43:1 sobre o fundo; escurecido para atingir AA (ver plano 6.1).
    expect(t["muted-foreground"].toUpperCase()).toBe("#5B6B82");
    expect(t.primary.toUpperCase()).toBe("#0F766E");
  });

  it("os pares de texto atingem o contraste AA (4,5:1)", () => {
    const pairs: [string, string, string][] = [
      ["foreground", "background", t.foreground + "|" + t.background],
      ["foreground", "card", t.foreground + "|" + t.card],
      ["muted-foreground", "card", t["muted-foreground"] + "|" + t.card],
      ["muted-foreground", "background", t["muted-foreground"] + "|" + t.background],
      ["primary-foreground", "primary", t["primary-foreground"] + "|" + t.primary],
      ["primary-foreground", "primary-hover", t["primary-foreground"] + "|" + t["primary-hover"]],
      ["primary", "primary-subtle", t.primary + "|" + t["primary-subtle"]],
      ["primary", "primary-selected", t.primary + "|" + t["primary-selected"]],
      ...SEMANTIC.map((s): [string, string, string] => [`${s}-foreground`, `${s}-subtle`, t[`${s}-foreground`] + "|" + t[`${s}-subtle`]]),
      ...STATES.map((s): [string, string, string] => [`state-${s}-fg`, `state-${s}-bg`, t[`state-${s}-fg`] + "|" + t[`state-${s}-bg`]]),
    ];
    for (const [fg, bg, joined] of pairs) {
      const [a, b] = joined.split("|");
      expect(contrastRatio(a, b), `${fg} sobre ${bg}`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("o anel de foco é visível sobre as superfícies (3:1)", () => {
    expect(contrastRatio(t.ring, t.card)).toBeGreaterThanOrEqual(3);
    expect(contrastRatio(t.ring, t.background)).toBeGreaterThanOrEqual(3);
  });

  it("cada estado do quarto tem cores próprias (nenhum repete outro)", () => {
    const backgrounds = STATES.map((s) => t[`state-${s}-bg`].toUpperCase());
    expect(new Set(backgrounds).size).toBe(STATES.length);
  });

  it("raios e sombras seguem o brief", () => {
    expect(t.radius).toBe("0.5rem");
    expect(t["radius-card"]).toBe("0.75rem");
    expect(t["shadow-card"]).toMatch(/rgb|#/);
    expect(t["shadow-overlay"]).toMatch(/rgb|#/);
  });

  it("a fonte é a Onest empacotada e nada é carregado da rede", () => {
    expect(css).toContain('@import "@fontsource-variable/onest"');
    expect(css).toMatch(/--font-sans:\s*'Onest Variable'/);
    expect(css).toContain("material-symbols/rounded.css");
    for (const source of [css, html]) {
      expect(source).not.toMatch(/https?:\/\//);
      expect(source).not.toMatch(/fonts\.(googleapis|gstatic)/);
    }
  });

  it("números ficam alinhados em tabelas", () => {
    expect(css).toMatch(/td,\s*th\s*\{[^}]*font-variant-numeric:\s*tabular-nums/);
  });
});
