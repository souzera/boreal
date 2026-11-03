import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import { ConfirmDialog } from "./ConfirmDialog";

function setup(open = true) {
  const onConfirm = vi.fn();
  const onCancel = vi.fn();
  render(
    <ConfirmDialog
      open={open}
      title="Desativar quarto?"
      description="O quarto deixa de aparecer nas novas reservas."
      confirmLabel="Desativar quarto"
      onConfirm={onConfirm}
      onCancel={onCancel}
    />,
  );
  return { onConfirm, onCancel };
}

describe("ConfirmDialog", () => {
  it("mostra título, descrição e o rótulo específico da ação", () => {
    setup();

    expect(screen.getByText("Desativar quarto?")).toBeInTheDocument();
    expect(screen.getByText("O quarto deixa de aparecer nas novas reservas.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Desativar quarto" })).toBeInTheDocument();
  });

  it("confirmar chama onConfirm; cancelar chama onCancel", async () => {
    const { onConfirm, onCancel } = setup();

    await userEvent.click(screen.getByRole("button", { name: "Desativar quarto" }));
    expect(onConfirm).toHaveBeenCalledTimes(1);

    await userEvent.click(screen.getByRole("button", { name: "Cancelar" }));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("fechado não renderiza nada", () => {
    setup(false);

    expect(screen.queryByText("Desativar quarto?")).toBeNull();
  });
});
