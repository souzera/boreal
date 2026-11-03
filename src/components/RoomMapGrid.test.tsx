import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { vi } from "vitest";

import type { RoomMap } from "@/lib/api";
import { RoomMapGrid } from "./RoomMapGrid";

const data: RoomMap = {
  from: "2099-10-24",
  to: "2099-10-30",
  rooms: [
    { id: 1, code: "101", category_name: "Standard", active: true, state: "available", state_label: "Disponível", free_at: null },
    { id: 2, code: "102", category_name: "Luxo", active: false, state: null, state_label: null, free_at: null },
  ],
  cells: [
    { room_id: 1, date: "2099-10-25", reservation_id: 5, code: "RES-000005", guest_name: "Ana Souza", status: "confirmed", is_check_in: true, is_check_out: false },
  ],
};

function show(props: Partial<React.ComponentProps<typeof RoomMapGrid>> = {}) {
  const onOpenReservation = vi.fn();
  render(
    <MemoryRouter>
      <RoomMapGrid data={data} from="2099-10-24" days={7} today="2099-10-24" onOpenReservation={onOpenReservation} {...props} />
    </MemoryRouter>,
  );
  return { onOpenReservation };
}

describe("RoomMapGrid", () => {
  it("mostra os dias, o hóspede e a situação em texto e abre a reserva", async () => {
    const { onOpenReservation } = show();
    const table = screen.getByRole("table");
    expect(within(table).getAllByRole("columnheader")).toHaveLength(8);

    await userEvent.click(within(table).getByRole("button", { name: /Ana Souza.*Confirmada.*chegada/i }));

    expect(onOpenReservation).toHaveBeenCalledWith(5);
  });

  it("marca o quarto inativo e só oferece Livre com onBook, de hoje em diante", async () => {
    const onBook = vi.fn();
    show({ onBook });
    expect(screen.getByRole("rowheader", { name: /102.*\(inativo\)/ })).toBeInTheDocument();
    expect(screen.queryAllByRole("button", { name: /Reservar quarto 102/ })).toHaveLength(0);

    await userEvent.click(screen.getByRole("button", { name: "Reservar quarto 101 em 24/10/2099" }));

    expect(onBook).toHaveBeenCalledWith(1, "2099-10-24");
  });

  it("não oferece Livre sem onBook nem em dias passados", () => {
    show({ today: "2099-10-26" });
    expect(screen.queryByRole("button", { name: /Reservar quarto/ })).not.toBeInTheDocument();
  });

  it("transforma o quarto ativo em botão quando recebe onOpenRoom; o inativo continua texto", async () => {
    const onOpenRoom = vi.fn();
    show({ onOpenRoom });
    expect(screen.queryByRole("button", { name: /102 · Luxo/ })).toBeNull();

    await userEvent.click(screen.getByRole("button", { name: /101 · Standard/ }));

    expect(onOpenRoom).toHaveBeenCalledWith(1);
  });
});
