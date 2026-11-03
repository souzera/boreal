import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createMemoryRouter, Link, RouterProvider } from "react-router-dom";
import { vi } from "vitest";

import { UnsavedChangesGuard } from "@/components/UnsavedChangesGuard";

function setup(isDirty: boolean, onSave?: () => Promise<boolean>) {
  const router = createMemoryRouter(
    [
      {
        path: "/",
        element: (
          <>
            <p>Página A</p>
            <UnsavedChangesGuard isDirty={isDirty} onSave={onSave} />
            <Link to="/b">Ir para B</Link>
          </>
        ),
      },
      { path: "/b", element: <p>Página B</p> },
    ],
    { initialEntries: ["/"] },
  );
  render(<RouterProvider router={router} />);
  return router;
}

describe("UnsavedChangesGuard", () => {
  it("sem alterações a navegação segue direto", async () => {
    setup(false);

    await userEvent.click(screen.getByText("Ir para B"));

    expect(await screen.findByText("Página B")).toBeInTheDocument();
  });

  it("com alterações pede decisão; Continuar editando mantém a página", async () => {
    setup(true);

    await userEvent.click(screen.getByText("Ir para B"));
    expect(await screen.findByText("Alterações não salvas")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Continuar editando" }));

    await waitFor(() => expect(screen.queryByText("Alterações não salvas")).toBeNull());
    expect(screen.getByText("Página A")).toBeInTheDocument();
  });

  it("Descartar segue para a página pedida", async () => {
    setup(true);

    await userEvent.click(screen.getByText("Ir para B"));
    await userEvent.click(await screen.findByRole("button", { name: "Descartar" }));

    expect(await screen.findByText("Página B")).toBeInTheDocument();
  });

  it("Salvar só aparece com onSave e só segue se o salvamento der certo", async () => {
    const onSave = vi.fn().mockResolvedValueOnce(false).mockResolvedValueOnce(true);
    setup(true, onSave);

    await userEvent.click(screen.getByText("Ir para B"));
    await userEvent.click(await screen.findByRole("button", { name: "Salvar" }));
    expect(onSave).toHaveBeenCalledTimes(1);
    expect(screen.getByText("Página A")).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "Salvar" }));
    expect(await screen.findByText("Página B")).toBeInTheDocument();
  });

  it("sem onSave não há botão Salvar", async () => {
    setup(true);

    await userEvent.click(screen.getByText("Ir para B"));
    await screen.findByText("Alterações não salvas");

    expect(screen.queryByRole("button", { name: "Salvar" })).toBeNull();
  });
});
