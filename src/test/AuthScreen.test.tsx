import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { AuthScreen } from "../components/AuthScreen";
import { LocalAuthProvider } from "../infrastructure/local/LocalAuthProvider";

afterEach(cleanup);

describe("AuthScreen", () => {
  it("signs in in demo mode", async () => {
    localStorage.clear();
    let email = "";
    render(<AuthScreen auth={new LocalAuthProvider()} mode="demo" onAuthenticated={(user) => { email = user.email; }} />);
    fireEvent.click(screen.getByRole("button", { name: "Войти" }));
    await waitFor(() => expect(email).toBe("player@example.com"));
  });

  it("signs in locally while clearly labeling field-test mode", async () => {
    localStorage.clear();
    let email = "";
    render(<AuthScreen auth={new LocalAuthProvider()} mode="field-test" onAuthenticated={(user) => { email = user.email; }} />);
    expect(screen.getByText(/ПОЛЕВОЙ ТЕСТ — профили хранятся на этом ПК/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Войти" }));
    await waitFor(() => expect(email).toBe("player@example.com"));
  });
});
