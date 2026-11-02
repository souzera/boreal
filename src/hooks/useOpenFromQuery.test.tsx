import { render, screen } from "@testing-library/react";
import { MemoryRouter, useLocation } from "react-router-dom";
import { vi } from "vitest";

import { useOpenFromQuery } from "./useOpenFromQuery";

function Probe({ ready, onOpen }: { ready: boolean; onOpen: (a: "checkin" | "pagamento") => boolean }) {
  useOpenFromQuery(["checkin", "pagamento"] as const, ready, onOpen);
  return <p data-testid="search">{useLocation().search}</p>;
}

function show(url: string, ready = true, onOpen = vi.fn(() => true)) {
  const view = render(
    <MemoryRouter initialEntries={[url]}>
      <Probe ready={ready} onOpen={onOpen} />
    </MemoryRouter>,
  );
  return { onOpen, view };
}

describe("useOpenFromQuery", () => {
  it("abre uma vez e limpa o endereço", () => {
    const { onOpen } = show("/x?abrir=checkin&aba=a");

    expect(onOpen).toHaveBeenCalledTimes(1);
    expect(onOpen).toHaveBeenCalledWith("checkin");
    expect(screen.getByTestId("search")).toHaveTextContent("?aba=a");
  });

  it("ignora e limpa valores desconhecidos", () => {
    const { onOpen } = show("/x?abrir=lixo");

    expect(onOpen).not.toHaveBeenCalled();
    expect(screen.getByTestId("search")).toBeEmptyDOMElement();
  });

  it("não abre antes de a página ter os dados", () => {
    const { onOpen, view } = show("/x?abrir=pagamento", false);
    expect(onOpen).not.toHaveBeenCalled();
    expect(screen.getByTestId("search")).toHaveTextContent("?abrir=pagamento");

    view.rerender(
      <MemoryRouter initialEntries={["/x?abrir=pagamento"]}>
        <Probe ready onOpen={onOpen} />
      </MemoryRouter>,
    );
  });

  it("limpa o parâmetro também quando a ação não cabe mais", () => {
    const { onOpen } = show("/x?abrir=checkin", true, vi.fn(() => false));

    expect(onOpen).toHaveBeenCalledTimes(1);
    expect(screen.getByTestId("search")).toBeEmptyDOMElement();
  });
});
