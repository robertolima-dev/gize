import { useState } from "react";
import { api } from "./api";
import { Icon } from "./icons";

export function useAuth() {
  const [token, setToken] = useState<string | null>(() => localStorage.getItem("gize_token"));
  function login(t: string) {
    localStorage.setItem("gize_token", t);
    setToken(t);
  }
  function logout() {
    localStorage.removeItem("gize_token");
    setToken(null);
  }
  return { token, login, logout };
}

export function Login({ onLogin }: { onLogin: (token: string) => void }) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [show, setShow] = useState(false);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError("");
    setBusy(true);
    try {
      const res = await api.login(email, password);
      onLogin(res.token);
    } catch {
      setError("Invalid email or password.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="login">
      <aside className="login__aside">
        <div className="brand">
          <span className="mark">g</span>
          <span>
            Gize <small>admin</small>
          </span>
        </div>
        <svg className="glyph" viewBox="0 0 100 100" fill="none" aria-hidden="true">
          <path d="M50 8 L92 88 H8 Z" stroke="currentColor" strokeWidth="2" />
          <path
            d="M50 8 L50 88 M30 50 L70 50 M20 69 L80 69"
            stroke="currentColor"
            strokeWidth="1.2"
            opacity=".6"
          />
        </svg>
        <div>
          <h2>Manage your app on solid foundations.</h2>
          <p>Sign in to the admin for your project. Generated from your manifest, as code you own.</p>
          <ul>
            <li>
              <Icon name="check" /> List, create, edit and delete every resource
            </li>
            <li>
              <Icon name="check" /> Filters, search and pagination out of the box
            </li>
            <li>
              <Icon name="check" /> Argon2 + JWT auth, same as your API
            </li>
          </ul>
        </div>
        <div className="login__note">Powered by Gize</div>
      </aside>

      <div className="login__main">
        <div className="login__card">
          <h1>Sign in</h1>
          <p>Use your account for this project.</p>
          <form onSubmit={submit}>
            <div className="form-row">
              <label className="label" htmlFor="email">
                Email
              </label>
              <input
                id="email"
                type="email"
                autoComplete="username"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
              />
            </div>
            <div className="form-row">
              <label className="label" htmlFor="pw">
                Password
              </label>
              <div className="pw-wrap">
                <input
                  id="pw"
                  type={show ? "text" : "password"}
                  autoComplete="current-password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                />
                <button type="button" onClick={() => setShow((s) => !s)} aria-label="Show password">
                  <Icon name="eye" />
                </button>
              </div>
            </div>
            {error && <p className="login__err">{error}</p>}
            <button
              className="btn primary"
              type="submit"
              disabled={busy}
              style={{ width: "100%", height: 38, justifyContent: "center", marginTop: 6 }}
            >
              {busy ? "Signing in…" : "Sign in"}
            </button>
          </form>
          <div className="login__foot">Powered by Gize</div>
        </div>
      </div>
    </div>
  );
}
