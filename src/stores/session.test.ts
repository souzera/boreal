import { useSessionStore, type User } from "./session";

const user: User = {
  id: 1,
  username: "admin",
  display_name: "Administrador",
  role: "admin",
  active: true,
  must_change_password: false,
  failed_attempts: 0,
  locked_until: null,
  version: 1,
};

beforeEach(() => useSessionStore.getState().clear());

describe("useSessionStore", () => {
  it("começa anônimo", () => {
    expect(useSessionStore.getState().state).toBe("anonymous");
  });

  it("setSession ativa a sessão e guarda a exigência de troca de senha", () => {
    useSessionStore.getState().setSession({ token: "t", user, must_change_password: true });

    const s = useSessionStore.getState();
    expect(s.state).toBe("active");
    expect(s.token).toBe("t");
    expect(s.mustChangePassword).toBe(true);
  });

  it("lock() bloqueia preservando o usuário", () => {
    useSessionStore.getState().setSession({ token: "t", user, must_change_password: false });
    useSessionStore.getState().lock();

    const s = useSessionStore.getState();
    expect(s.state).toBe("locked");
    expect(s.user).toEqual(user);
  });

  it("clear() volta a anônimo e descarta token e usuário", () => {
    useSessionStore.getState().setSession({ token: "t", user, must_change_password: false });
    useSessionStore.getState().clear();

    const s = useSessionStore.getState();
    expect(s.state).toBe("anonymous");
    expect(s.token).toBeNull();
    expect(s.user).toBeNull();
  });
});
