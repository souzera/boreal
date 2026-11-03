import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import DocumentActions from "./DocumentActions";

const save = vi.fn();
vi.mock("@tauri-apps/plugin-dialog", () => ({ save: (...args: unknown[]) => save(...args) }));

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, listDocuments: vi.fn(), issueDocument: vi.fn(), saveDocumentPdf: vi.fn(), openExportedFile: vi.fn() };
});

function doc(version: number, current: boolean): api.DocumentView {
  return {
    id: 10 + version, kind: "reservation_confirmation", kind_label: "Confirmação de reserva", source_id: 5, version,
    issued_at: "2026-10-21T17:05:00Z", issued_by_name: "Ana", current,
  };
}

function show() {
  return renderPage(<DocumentActions kind="reservation_confirmation" sourceId={5} label="Emitir confirmação de reserva" fileStem="confirmacao-RES-000005" />);
}

beforeEach(() => {
  signIn();
  save.mockReset();
  vi.mocked(api.listDocuments).mockReset().mockResolvedValue([]);
  vi.mocked(api.issueDocument).mockReset();
  vi.mocked(api.saveDocumentPdf).mockReset().mockResolvedValue({ path: "C:/x/d.pdf", bytes: 5 });
  vi.mocked(api.openExportedFile).mockReset().mockResolvedValue(null);
});

describe("DocumentActions", () => {
  it("sem versões mostra só o botão de emitir", async () => {
    show();

    expect(await screen.findByRole("button", { name: "Emitir confirmação de reserva" })).toBeInTheDocument();
    expect(screen.queryByRole("list", { name: "Versões emitidas" })).toBeNull();
  });

  it("emitir chama a API uma vez, avisa a versão e atualiza a lista", async () => {
    vi.mocked(api.issueDocument).mockResolvedValue(doc(1, true));
    show();
    await screen.findByRole("button", { name: "Emitir confirmação de reserva" });
    vi.mocked(api.listDocuments).mockResolvedValue([doc(1, true)]);

    await userEvent.click(screen.getByRole("button", { name: "Emitir confirmação de reserva" }));

    expect(await screen.findByText("Documento emitido: versão 1.")).toBeInTheDocument();
    expect(api.issueDocument).toHaveBeenCalledTimes(1);
    expect(api.issueDocument).toHaveBeenCalledWith("t", "reservation_confirmation", 5);
    expect(await screen.findByText(/Versão 1 · emitida em .* por Ana · vigente/)).toBeInTheDocument();
  });

  it("emitir sem mudanças avisa que a versão continua valendo", async () => {
    vi.mocked(api.listDocuments).mockResolvedValue([doc(1, true)]);
    vi.mocked(api.issueDocument).mockResolvedValue(doc(1, true));
    show();
    await screen.findByRole("list", { name: "Versões emitidas" });

    await userEvent.click(screen.getByRole("button", { name: "Emitir confirmação de reserva" }));

    expect(await screen.findByText("Nada mudou desde a versão 1: ela continua valendo.")).toBeInTheDocument();
  });

  it("lista as versões da mais nova para a mais antiga, com a vigente marcada em texto", async () => {
    vi.mocked(api.listDocuments).mockResolvedValue([doc(2, true), doc(1, false)]);
    show();

    const items = await screen.findAllByRole("listitem");
    expect(items[0]).toHaveTextContent("Versão 2");
    expect(items[0]).toHaveTextContent("vigente");
    expect(items[1]).toHaveTextContent("Versão 1");
    expect(items[1]).not.toHaveTextContent("vigente");
  });

  it("Salvar PDF abre o diálogo com o nome sugerido, grava e oferece abrir o arquivo", async () => {
    vi.mocked(api.listDocuments).mockResolvedValue([doc(2, true)]);
    save.mockResolvedValue("C:/x/d.pdf");
    show();

    await userEvent.click(await screen.findByRole("button", { name: "Salvar PDF da versão 2" }));

    expect(save).toHaveBeenCalledWith(expect.objectContaining({ defaultPath: "confirmacao-RES-000005-v2.pdf" }));
    await waitFor(() => expect(api.saveDocumentPdf).toHaveBeenCalledWith("t", 12, "C:/x/d.pdf"));
    expect(await screen.findByText("PDF salvo em C:/x/d.pdf")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Abrir arquivo" }));
    expect(api.openExportedFile).toHaveBeenCalledWith("t", "C:/x/d.pdf");
  });

  it("cancelar o diálogo não faz nada", async () => {
    vi.mocked(api.listDocuments).mockResolvedValue([doc(1, true)]);
    save.mockResolvedValue(null);
    show();

    await userEvent.click(await screen.findByRole("button", { name: "Salvar PDF da versão 1" }));

    expect(api.saveDocumentPdf).not.toHaveBeenCalled();
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("falha ao gravar mostra a mensagem e Tentar de novo repete só a gravação, sem emitir de novo", async () => {
    vi.mocked(api.listDocuments).mockResolvedValue([doc(1, true)]);
    save.mockResolvedValue("C:/x/d.pdf");
    vi.mocked(api.saveDocumentPdf)
      .mockRejectedValueOnce({ code: "validation", message: "Não foi possível gravar neste local. Escolha outra pasta e tente de novo." })
      .mockResolvedValueOnce({ path: "C:/x/d.pdf", bytes: 5 });
    show();

    await userEvent.click(await screen.findByRole("button", { name: "Salvar PDF da versão 1" }));
    expect(await screen.findByText(/Escolha outra pasta/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Tentar de novo" }));

    expect(await screen.findByText("PDF salvo em C:/x/d.pdf")).toBeInTheDocument();
    expect(api.saveDocumentPdf).toHaveBeenCalledTimes(2);
    expect(save).toHaveBeenCalledTimes(1);
    expect(api.issueDocument).not.toHaveBeenCalled();
  });

  it("falha do diálogo Salvar como mostra a mensagem e não grava nada", async () => {
    vi.mocked(api.listDocuments).mockResolvedValue([doc(1, true)]);
    save.mockRejectedValue(new Error("boom"));
    show();

    await userEvent.click(await screen.findByRole("button", { name: "Salvar PDF da versão 1" }));

    expect(await screen.findByText("Não foi possível abrir a janela de salvar. Tente de novo.")).toBeInTheDocument();
    expect(api.saveDocumentPdf).not.toHaveBeenCalled();
  });

  it("erro ao emitir mostra a mensagem do backend", async () => {
    vi.mocked(api.issueDocument).mockRejectedValue({ code: "validation", message: "O demonstrativo final só pode ser emitido depois do check-out." });
    show();

    await userEvent.click(await screen.findByRole("button", { name: "Emitir confirmação de reserva" }));

    expect(await screen.findByText(/depois do check-out/)).toBeInTheDocument();
  });
});
