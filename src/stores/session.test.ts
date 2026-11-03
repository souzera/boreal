import { queryClient } from "@/lib/queryClient";
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

  it("expire() mantém a tela montada (estado expired) e preserva o usuário", () => {
    useSessionStore.getState().setSession({ token: "t", user, must_change_password: false });
    useSessionStore.getState().expire();

    const s = useSessionStore.getState();
    expect(s.state).toBe("expired");
    expect(s.user).toEqual(user);
  });

  it("expire() não faz nada sem sessão", () => {
    useSessionStore.getState().expire();

    expect(useSessionStore.getState().state).toBe("anonymous");
  });

  it("setSession volta de expired para active com o novo token", () => {
    useSessionStore.getState().setSession({ token: "t", user, must_change_password: false });
    useSessionStore.getState().expire();
    useSessionStore.getState().setSession({ token: "t2", user, must_change_password: false });

    expect(useSessionStore.getState().state).toBe("active");
    expect(useSessionStore.getState().token).toBe("t2");
  });

  it("guarda o tempo de inatividade vindo do backend", () => {
    useSessionStore.getState().setIdleMinutes(3);

    expect(useSessionStore.getState().idleMinutes).toBe(3);
  });

  it("clear() esvazia o cache de consultas (dados de quem saiu)", () => {
    queryClient.setQueryData(["guest", 7], { full_name: "Ana" });
    useSessionStore.getState().setSession({ token: "t", user, must_change_password: false });

    useSessionStore.getState().clear();

    expect(queryClient.getQueryData(["guest", 7])).toBeUndefined();
  });

  it("entrar com outro usuário esvazia o cache; o mesmo usuário (novo login) mantém", () => {
    useSessionStore.getState().setSession({ token: "t", user, must_change_password: false });
    queryClient.setQueryData(["guest", 7], { full_name: "Ana" });

    useSessionStore.getState().setSession({ token: "t2", user, must_change_password: false });
    expect(queryClient.getQueryData(["guest", 7])).toBeDefined();

    useSessionStore.getState().setSession({ token: "t3", user: { ...user, id: 2 }, must_change_password: false });
    expect(queryClient.getQueryData(["guest", 7])).toBeUndefined();
  });
});
