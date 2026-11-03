import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import { SidePanel } from "./SidePanel";

function show() {
  const onClose = vi.fn();
  render(
    <div>
      <p>Agenda atrás do painel</p>
      <SidePanel title="Reserva RES-000005" onClose={onClose}>
        <button type="button">Dentro</button>
      </SidePanel>
    </div>,
  );
  return onClose;
}

describe("SidePanel", () => {
  it("é um painel nomeado e não modal", () => {
    show();

    expect(screen.getByRole("complementary", { name: "Reserva RES-000005" })).toBeInTheDocument();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByText("Agenda atrás do painel")).toBeVisible();
    expect(screen.getByText("Agenda atrás do painel").closest("[aria-hidden]")).toBeNull();
  });

  it("fecha pelo botão", async () => {
    const onClose = show();

    await userEvent.click(screen.getByRole("button", { name: "Fechar painel" }));

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("recebe o foco ao abrir, então Esc fecha sem precisar clicar nele", async () => {
    const onClose = show();

    expect(screen.getByRole("complementary", { name: "Reserva RES-000005" })).toHaveFocus();
    await userEvent.keyboard("{Escape}");

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("Esc com o foco fora do painel não o fecha", async () => {
    const onClose = show();
    screen.getByText("Agenda atrás do painel").focus();
    (document.activeElement as HTMLElement | null)?.blur();

    await userEvent.keyboard("{Escape}");

    expect(onClose).not.toHaveBeenCalled();
  });
});
