import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useLocation } from "react-router-dom";
import { vi } from "vitest";

import GlobalSearch from "@/components/layout/GlobalSearch";
import * as api from "@/lib/api";
import { renderPage, signIn } from "@/test/render";
import Search from "./Search";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return { ...actual, globalSearch: vi.fn() };
});

const group = <T,>(items: T[], total = items.length) => ({ items, total });

const results = (patch: Partial<api.SearchResults> = {}): api.SearchResults => ({
  query: "ana",
  guests: group([{ id: 7, code: "HOS-000007", name: "Ana Souza", phone: "11999990000", cpf_masked: "***.***.***-25", active: true }]),
  reservations: group([{ id: 5, code: "RES-000005", guest_name: "Ana Souza", room_code: "101", check_in_date: "2026-10-21", check_out_date: "2026-10-23", status: "confirmed" as const }]),
  stays: group([{ id: 3, code: "RES-000005", guest_name: "Ana Souza", room_code: "101", status: "in_house" as const }]),
  movements: group([{ kind: "payment" as const, code: "PAG-000001", label: "Recebimento PAG-000001 · Ana Souza", amount_cents: 12000, at: "2026-10-21T15:00:00Z", link: "/hospedagens/3" }]),
  ...patch,
});

const empty = results({ guests: group([]), reservations: group([]), stays: group([]), movements: group([]) });

beforeEach(() => {
  signIn();
  vi.mocked(api.globalSearch).mockReset().mockResolvedValue(results());
});

function Where() {
  const location = useLocation();
  return <p>{`${location.pathname}${location.search}`}</p>;
}

describe("GlobalSearch", () => {
  it("enviar a busca leva a /busca com o texto", async () => {
    renderPage(<GlobalSearch />, { extraRoutes: [{ path: "/busca", element: <Where /> }] });

    await userEvent.type(screen.getByRole("searchbox", { name: "Busca geral" }), "  ana souza ");
    await userEvent.keyboard("{Enter}");

    expect(await screen.findByText("/busca?q=ana+souza")).toBeInTheDocument();
  });

  it("busca vazia ou de um só caractere não navega", async () => {
    renderPage(<GlobalSearch />, { extraRoutes: [{ path: "/busca", element: <Where /> }] });
    const box = screen.getByRole("searchbox", { name: "Busca geral" });

    await userEvent.type(box, "   {Enter}");
    await userEvent.clear(box);
    await userEvent.type(box, "a{Enter}");

    expect(screen.queryByText(/\/busca/)).toBeNull();
  });
});

describe("Search page", () => {
  it("mostra os quatro grupos, cada um com link para o registro", async () => {
    renderPage(<Search />, { path: "/busca", entry: "/busca?q=ana" });

    expect(await screen.findByRole("heading", { name: /Hóspedes/ })).toBeInTheDocument();
    for (const name of [/Reservas/, /Hospedagens/, /Movimentos/]) expect(screen.getByRole("heading", { name })).toBeInTheDocument();
    const guests = within(screen.getByRole("region", { name: "Hóspedes" }));
    expect(guests.getByRole("link", { name: /Ana Souza/ })).toHaveAttribute("href", "/hospedes/7");
    expect(guests.getByText("***.***.***-25")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "Reservas" })).getByRole("link", { name: /RES-000005/ })).toHaveAttribute("href", "/reservas/5");
    expect(within(screen.getByRole("region", { name: "Hospedagens" })).getByRole("link", { name: /RES-000005/ })).toHaveAttribute("href", "/hospedagens/3");
    expect(within(screen.getByRole("region", { name: "Movimentos" })).getByRole("link", { name: /PAG-000001/ })).toHaveAttribute("href", "/hospedagens/3");
    await waitFor(() => expect(api.globalSearch).toHaveBeenCalledWith("t", "ana"));
  });

  it("reserva e hospedagem mostram a situação em ícone e texto", async () => {
    renderPage(<Search />, { path: "/busca", entry: "/busca?q=ana" });

    await screen.findByRole("region", { name: "Reservas" });
    const reservation = within(screen.getByRole("region", { name: "Reservas" })).getByRole("listitem");
    expect(reservation.querySelector("[data-icon]")).not.toBeNull();
    expect(reservation).toHaveTextContent("Confirmada");
    expect(reservation).toHaveTextContent("21/10/2026 a 23/10/2026");
  });

  it("grupo sem resultado some", async () => {
    vi.mocked(api.globalSearch).mockResolvedValue(results({ movements: group([]), stays: group([]) }));
    renderPage(<Search />, { path: "/busca", entry: "/busca?q=ana" });
    await screen.findByRole("heading", { name: /Hóspedes/ });

    expect(screen.queryByRole("heading", { name: /Hospedagens/ })).toBeNull();
    expect(screen.queryByRole("heading", { name: /Movimentos/ })).toBeNull();
  });

  it("quando há mais do que o mostrado, diz quantos", async () => {
    vi.mocked(api.globalSearch).mockResolvedValue(results({ guests: group([{ id: 7, code: "HOS-000007", name: "Ana Souza", phone: "11999990000", cpf_masked: null, active: true }], 12) }));
    renderPage(<Search />, { path: "/busca", entry: "/busca?q=ana" });

    expect(await screen.findByText("Mostrando 1 de 12. Refine a busca para ver os demais.")).toBeInTheDocument();
  });

  it("sem resultados orienta a pessoa com exemplos de busca", async () => {
    vi.mocked(api.globalSearch).mockResolvedValue(empty);
    renderPage(<Search />, { path: "/busca", entry: "/busca?q=zzz" });

    expect(await screen.findByText("Nenhum resultado para “zzz”.")).toBeInTheDocument();
    expect(screen.getByText(/RES-000012/)).toBeInTheDocument();
  });

  it("texto curto mostra a orientação e não chama o backend", async () => {
    renderPage(<Search />, { path: "/busca", entry: "/busca?q=a" });

    expect(await screen.findByText("Digite pelo menos 2 letras ou números no campo de busca.")).toBeInTheDocument();
    expect(api.globalSearch).not.toHaveBeenCalled();
  });

  it("sem texto também orienta", async () => {
    renderPage(<Search />, { path: "/busca", entry: "/busca" });

    expect(await screen.findByText(/Digite um nome, telefone, CPF, código ou número do quarto/)).toBeInTheDocument();
    expect(api.globalSearch).not.toHaveBeenCalled();
  });

  it("erro do backend aparece como mensagem", async () => {
    vi.mocked(api.globalSearch).mockRejectedValue(new api.ApiError("validation", "Digite pelo menos 2 letras ou números."));
    renderPage(<Search />, { path: "/busca", entry: "/busca?q=ab" });

    expect(await screen.findByRole("alert")).toHaveTextContent("Digite pelo menos 2 letras ou números.");
  });
});
