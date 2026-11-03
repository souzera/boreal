import { act, fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import { renderPage, signIn } from "@/test/render";
import { useSessionStore } from "@/stores/session";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
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
    expect(router.state.location.pathname + router.state.location.search).toBe("/?aba=chegadas");
    await userEvent.click(screen.getByRole("button", { name: /Receber pagamento/ }));
    expect(router.state.location.pathname).toBe("/hospedagens");
    await userEvent.click(screen.getByRole("button", { name: /^Check-out/ }));
    expect(router.state.location.pathname + router.state.location.search).toBe("/?aba=saidas");
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

  it("repetir o atalho na própria tela não empilha entradas no histórico", () => {
    const { router } = open();

    fireEvent.keyDown(window, { key: "n", code: "KeyN", altKey: true });
    const key = router.state.location.key;
    fireEvent.keyDown(window, { key: "n", code: "KeyN", altKey: true });

    expect(router.state.location.pathname).toBe("/reservas/nova");
    expect(router.state.location.key).toBe(key);
  });

  it("repetir Check-in na aba de chegadas não empilha entradas, mas trocar de aba navega", () => {
    const { router } = open();

    fireEvent.keyDown(window, { key: "c", code: "KeyC", altKey: true });
    const key = router.state.location.key;
    fireEvent.keyDown(window, { key: "c", code: "KeyC", altKey: true });
    expect(router.state.location.key).toBe(key);

    fireEvent.keyDown(window, { key: "s", code: "KeyS", altKey: true });
    expect(router.state.location.search).toBe("?aba=saidas");
  });

  it("com um diálogo de verdade aberto, nem os atalhos nem Ctrl+K fazem nada", async () => {
    const { router } = renderPage(
      <>
        <GlobalActions onSearch={onSearch} />
        <Dialog open>
          <DialogContent>
            <DialogTitle>Confirmar</DialogTitle>
          </DialogContent>
        </Dialog>
      </>,
      { path: "*", entry: "/mapa" },
    );
    await screen.findByRole("dialog");

    fireEvent.keyDown(window, { key: "n", code: "KeyN", altKey: true });
    fireEvent.keyDown(window, { key: "k", code: "KeyK", ctrlKey: true });

    expect(router.state.location.pathname).toBe("/mapa");
    expect(onSearch).not.toHaveBeenCalled();
  });

  it("em janelas estreitas só o ícone aparece, mas o nome da ação continua para leitores de tela", () => {
    open();

    const label = screen.getByRole("button", { name: /Receber pagamento/ }).querySelector("span:not([data-icon])");
    expect(label).toHaveTextContent("Receber pagamento");
    expect(label?.className).toContain("max-xl:sr-only");
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
