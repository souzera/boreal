import { act, render } from "@testing-library/react";
import { vi } from "vitest";

import * as api from "@/lib/api";
import { useActivityHeartbeat } from "./useActivityHeartbeat";

vi.mock("@/lib/api", () => ({ sessionStatus: vi.fn().mockResolvedValue({ state: "active", user: null }) }));

function Probe({ enabled = true }: { enabled?: boolean }) {
  useActivityHeartbeat("token-1", enabled);
  return null;
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date("2026-10-21T12:00:00Z"));
  vi.mocked(api.sessionStatus).mockClear();
});
afterEach(() => vi.useRealTimers());

describe("useActivityHeartbeat", () => {
  it("avisa o backend da atividade, no máximo uma vez a cada 30 segundos", () => {
    render(<Probe />);

    act(() => void window.dispatchEvent(new Event("mousemove")));
    act(() => void window.dispatchEvent(new Event("keydown")));
    expect(api.sessionStatus).toHaveBeenCalledTimes(1);
    expect(api.sessionStatus).toHaveBeenCalledWith("token-1");

    vi.setSystemTime(new Date("2026-10-21T12:00:31Z"));
    act(() => void window.dispatchEvent(new Event("mousemove")));
    expect(api.sessionStatus).toHaveBeenCalledTimes(2);
  });

  it("não envia nada quando desabilitado", () => {
    render(<Probe enabled={false} />);

    act(() => void window.dispatchEvent(new Event("mousemove")));

    expect(api.sessionStatus).not.toHaveBeenCalled();
  });
});
