import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

const win = {
  minimize: vi.fn().mockResolvedValue(undefined),
  toggleMaximize: vi.fn().mockResolvedValue(undefined),
  close: vi.fn().mockResolvedValue(undefined),
  isMaximized: vi.fn().mockResolvedValue(false),
  onResized: vi.fn(),
};
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => win }));

import TitleBar from "./TitleBar";

let resized: () => void = () => {};
const unlisten = vi.fn();

beforeEach(() => {
  win.minimize.mockClear();
  win.toggleMaximize.mockClear();
  win.close.mockClear();
  unlisten.mockClear();
  win.isMaximized.mockReset().mockResolvedValue(false);
  win.onResized.mockReset().mockImplementation((handler: () => void) => {
    resized = handler;
    return Promise.resolve(unlisten);
  });
});
afterEach(() => {
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  document.documentElement.style.removeProperty("--titlebar-h");
});

const insideTauri = () => {
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
};

describe("TitleBar", () => {
  it("não aparece fora do aplicativo (navegador e testes)", () => {
    render(<TitleBar />);

    expect(screen.queryByRole("toolbar", { name: "Janela" })).toBeNull();
    expect(document.documentElement.style.getPropertyValue("--titlebar-h")).toBe("");
  });

  it("mostra o nome, é uma área de arrastar e reserva a altura no CSS", async () => {
    insideTauri();
    render(<TitleBar />);

    const bar = await screen.findByRole("toolbar", { name: "Janela" });
    expect(bar).toHaveAttribute("data-tauri-drag-region");
    expect(bar).toHaveTextContent("Boreal PMS");
    expect(document.documentElement.style.getPropertyValue("--titlebar-h")).toBe("36px");
  });

  it("cada botão tem nome e chama a ação da janela", async () => {
    insideTauri();
    render(<TitleBar />);

    await userEvent.click(await screen.findByRole("button", { name: "Minimizar" }));
    await userEvent.click(screen.getByRole("button", { name: "Maximizar" }));
    await userEvent.click(screen.getByRole("button", { name: "Fechar" }));

    expect(win.minimize).toHaveBeenCalledTimes(1);
    expect(win.toggleMaximize).toHaveBeenCalledTimes(1);
    expect(win.close).toHaveBeenCalledTimes(1);
  });

  it("o botão do meio vira Restaurar quando a janela está maximizada", async () => {
    insideTauri();
    win.isMaximized.mockResolvedValue(true);
    render(<TitleBar />);

    expect(await screen.findByRole("button", { name: "Restaurar" })).toBeInTheDocument();

    win.isMaximized.mockResolvedValue(false);
    act(() => resized());
    await waitFor(() => expect(screen.getByRole("button", { name: "Maximizar" })).toBeInTheDocument());
  });

  it("ao sair, solta o ouvinte da janela e devolve a altura ao CSS", async () => {
    insideTauri();
    const { unmount } = render(<TitleBar />);
    await screen.findByRole("toolbar", { name: "Janela" });

    unmount();

    await waitFor(() => expect(unlisten).toHaveBeenCalledTimes(1));
    expect(document.documentElement.style.getPropertyValue("--titlebar-h")).toBe("0px");
  });
});
