import { act, fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import { renderPage, signIn } from "@/test/render";
import { useSessionStore } from "@/stores/session";
import GlobalActions from "./GlobalActions";

const onSearch = vi.fn();

function open() {
  return renderPage(<GlobalActions onSearch={onSearch} />, { path: "*", entry: "/mapa" });
}

beforeEach(() => {
  signIn();
  onSearch.mockReset();
});

describe("GlobalActions", () => {
  it("mostra as cinco ações com ícone e texto", () => {
    open();

    for (const label of ["Nova reserva", "Buscar", "Check-in", "Receber pagamento", "Check-out"]) {
      const button = screen.getByRole("button", { name: new RegExp(label) });
      expect(button.querySelector("[data-icon]"), label).not.toBeNull();
    }
  });

  it("cada ação leva ao lugar certo", async () => {
    const { router } = open();

    await userEvent.click(screen.getByRole("button", { name: /Nova reserva/ }));
    expect(router.state.location.pathname).toBe("/reservas/nova");
    await userEvent.click(screen.getByRole("button", { name: /^Check-in/ }));
    expect(router.state.location.pathname).toBe("/reservas");
    await userEvent.click(screen.getByRole("button", { name: /Receber pagamento/ }));
    expect(router.state.location.pathname).toBe("/hospedagens");
    await userEvent.click(screen.getByRole("button", { name: /^Check-out/ }));
    expect(router.state.location.pathname).toBe("/hospedagens");
  });

  it("Buscar chama o foco da busca", async () => {
    open();

    await userEvent.click(screen.getByRole("button", { name: /Buscar/ }));

    expect(onSearch).toHaveBeenCalledTimes(1);
  });

  it("anuncia o atalho de cada ação", () => {
    open();

    expect(screen.getByRole("button", { name: /Nova reserva/ })).toHaveAttribute("aria-keyshortcuts", "Alt+N");
    expect(screen.getByRole("button", { name: /Buscar/ })).toHaveAttribute("aria-keyshortcuts", "Control+K");
    expect(screen.getByRole("button", { name: /Nova reserva/ })).toHaveAttribute("title", "Nova reserva (Alt+N)");
  });

  it("o atalho funciona na tela e some com a sessão bloqueada", () => {
    const { router } = open();

    fireEvent.keyDown(window, { key: "n", code: "KeyN", altKey: true });
    expect(router.state.location.pathname).toBe("/reservas/nova");

    act(() => useSessionStore.getState().lock());
    fireEvent.keyDown(window, { key: "r", code: "KeyR", altKey: true });
    expect(router.state.location.pathname).toBe("/reservas/nova");
  });
});
