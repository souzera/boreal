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

  it("fecha com Esc quando o foco está dentro, e só então", async () => {
    const onClose = show();

    await userEvent.keyboard("{Escape}");
    expect(onClose).not.toHaveBeenCalled();

    await userEvent.click(screen.getByRole("button", { name: "Dentro" }));
    await userEvent.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
