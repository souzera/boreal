import capabilitiesRaw from "../../src-tauri/capabilities/default.json?raw";
import tauriConfRaw from "../../src-tauri/tauri.conf.json?raw";

interface TauriConf {
  app: { security: { csp: string; devCsp: string }; windows: { decorations: boolean }[] };
}
const conf = JSON.parse(tauriConfRaw) as TauriConf;
const capabilities = JSON.parse(capabilitiesRaw) as { permissions: string[]; windows: string[] };

const directives = (csp: string) => Object.fromEntries(csp.split(";").map((d) => d.trim().split(/\s+/)).filter((p) => p[0]).map(([name, ...values]) => [name, values]));

describe("política de segurança de conteúdo (CSP)", () => {
  for (const [name, csp] of [["produção", conf.app.security.csp], ["desenvolvimento", conf.app.security.devCsp]] as const) {
    it(`${name}: restritiva, sem avaliação dinâmica de código e sem curinga`, () => {
      const d = directives(csp);

      expect(d["default-src"]).toEqual(["'self'"]);
      expect(d["object-src"]).toEqual(["'none'"]);
      expect(d["base-uri"]).toEqual(["'none'"]);
      expect(d["frame-ancestors"]).toEqual(["'none'"]);
      expect(d["form-action"]).toEqual(["'self'"]);
      expect(csp).not.toContain("unsafe-eval");
      expect(csp).not.toMatch(/(^|\s)\*(\s|;|$)/);
      expect(d["script-src"] ?? ["'self'"]).not.toContain("'unsafe-inline'");
    });
  }

  it("a produção só fala com o próprio aplicativo (sem rede externa)", () => {
    const connect = directives(conf.app.security.csp)["connect-src"];

    expect(connect).toEqual(["'self'", "ipc:", "http://ipc.localhost"]);
  });
});

describe("permissões do Tauri", () => {
  // Lista fechada: acrescentar uma permissão exige mudar este teste de propósito (e justificar no PR).
  const ALLOWED = [
    "core:default",
    "core:window:allow-minimize",
    "core:window:allow-toggle-maximize",
    "core:window:allow-is-maximized",
    "core:window:allow-close",
    "core:window:allow-start-dragging",
    "core:window:allow-internal-toggle-maximize",
    "dialog:allow-save",
    "dialog:allow-open",
  ];

  it("só as permissões da lista fechada existem, e só para a janela principal", () => {
    expect([...capabilities.permissions].sort()).toEqual([...ALLOWED].sort());
    expect(capabilities.windows).toEqual(["main"]);
  });

  it("não há permissão de sistema de arquivos, shell, rede ou abrir caminhos arbitrários", () => {
    for (const permission of capabilities.permissions) {
      expect(permission, permission).not.toMatch(/^(fs|shell|http|opener|process|os):/);
    }
  });
});
