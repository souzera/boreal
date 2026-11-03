import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";

import { RoomStateBadge } from "./RoomStateBadge";

describe("RoomStateBadge", () => {
  it("mostra ícone e rótulo, não só cor", () => {
    const { container } = render(<RoomStateBadge state="awaiting_cleaning" />);

    expect(container.querySelector('[data-icon="cleaning_services"]')).not.toBeNull();
    expect(screen.getByText("Aguardando limpeza")).toBeInTheDocument();
  });

  it("usa as cores do estado vindas do tema", () => {
    const { container } = render(<RoomStateBadge state="blocked" />);

    expect(container.firstChild).toHaveClass("bg-state-blocked-bg", "text-state-blocked-fg");
  });

  it("diz a que horas o quarto limpo fica livre", () => {
    render(<RoomStateBadge state="clean_waiting_interval" freeAt="2026-10-23T11:00" />);

    expect(screen.getByText("livre às 11:00")).toBeInTheDocument();
  });

  it("vira link quando recebe um destino", () => {
    render(
      <MemoryRouter>
        <RoomStateBadge state="available" to="/limpeza" />
      </MemoryRouter>,
    );

    expect(screen.getByRole("link", { name: /Disponível/ })).toHaveAttribute("href", "/limpeza");
  });
});
