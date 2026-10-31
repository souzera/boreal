import { fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";

import { SHORTCUTS, useGlobalShortcuts } from "./useGlobalShortcuts";

const onSearch = vi.fn();
const onNavigate = vi.fn();

function Harness({ enabled = true, withDialog = false }: { enabled?: boolean; withDialog?: boolean }) {
  useGlobalShortcuts({ onSearch, onNavigate, enabled });
  return (
    <div>
      <input aria-label="campo" />
      {withDialog && <div role="dialog" />}
    </div>
  );
}

const press = (init: KeyboardEventInit, target: Element | Window = window) => fireEvent.keyDown(target, init);

beforeEach(() => {
  onSearch.mockReset();
  onNavigate.mockReset();
});

describe("useGlobalShortcuts", () => {
  it("expõe os atalhos para serem mostrados na tela", () => {
    expect(SHORTCUTS).toEqual({
      search: "Ctrl+K",
      newReservation: "Alt+N",
      checkIn: "Alt+C",
      receivePayment: "Alt+R",
      checkOut: "Alt+S",
    });
  });

  it("Ctrl+K busca, mesmo com o foco num campo, e impede a ação padrão do navegador", () => {
    render(<Harness />);
    const field = screen.getByLabelText("campo");
    field.focus();

    const notCancelled = fireEvent.keyDown(field, { key: "k", code: "KeyK", ctrlKey: true });

    expect(onSearch).toHaveBeenCalledTimes(1);
    expect(notCancelled).toBe(false);
  });

  it("Alt+N, Alt+C, Alt+R e Alt+S levam à tela certa", () => {
    render(<Harness />);

    press({ key: "n", code: "KeyN", altKey: true });
    press({ key: "c", code: "KeyC", altKey: true });
    press({ key: "r", code: "KeyR", altKey: true });
    press({ key: "s", code: "KeyS", altKey: true });

    expect(onNavigate.mock.calls.map((c) => c[0])).toEqual(["/reservas/nova", "/reservas", "/hospedagens", "/hospedagens"]);
  });

  it("reconhece a tecla pelo código físico mesmo quando o Alt troca o caractere", () => {
    render(<Harness />);

    press({ key: "¬", code: "KeyN", altKey: true });

    expect(onNavigate).toHaveBeenCalledWith("/reservas/nova");
  });

  it("ignora tecla repetida, combinações com outros modificadores e teclas sem relação", () => {
    render(<Harness />);

    press({ key: "n", code: "KeyN", altKey: true, repeat: true });
    press({ key: "n", code: "KeyN", altKey: true, shiftKey: true });
    press({ key: "n", code: "KeyN", altKey: true, ctrlKey: true });
    press({ key: "k", code: "KeyK", ctrlKey: true, altKey: true });
    press({ key: "n", code: "KeyN" });
    press({ key: "x", code: "KeyX", altKey: true });

    expect(onNavigate).not.toHaveBeenCalled();
    expect(onSearch).not.toHaveBeenCalled();
  });

  it("não dispara com um diálogo aberto", () => {
    render(<Harness withDialog />);

    press({ key: "n", code: "KeyN", altKey: true });
    press({ key: "k", code: "KeyK", ctrlKey: true });

    expect(onNavigate).not.toHaveBeenCalled();
    expect(onSearch).not.toHaveBeenCalled();
  });

  it("não dispara quando desligado (sessão bloqueada)", () => {
    render(<Harness enabled={false} />);

    press({ key: "n", code: "KeyN", altKey: true });
    press({ key: "k", code: "KeyK", ctrlKey: true });

    expect(onNavigate).not.toHaveBeenCalled();
    expect(onSearch).not.toHaveBeenCalled();
  });

  it("para de ouvir quando o componente sai da tela", () => {
    const { unmount } = render(<Harness />);
    unmount();

    press({ key: "n", code: "KeyN", altKey: true });

    expect(onNavigate).not.toHaveBeenCalled();
  });
});
