import { useState } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { resources, type ResourceDesc } from "./resources";
import { Resource } from "./Resource";
import { ResourceForm } from "./ResourceForm";
import { Login, useAuth } from "./auth";
import { ToastHost } from "./toast";
import { titleCase } from "./api";
import { Icon } from "./icons";
import { currentTheme, toggleTheme, type Theme } from "./theme";

const client = new QueryClient();

type Editing = { desc: ResourceDesc; row: Record<string, any> | null };

export default function App() {
  const auth = useAuth();
  const [theme, setThemeState] = useState<Theme>(currentTheme());
  const [activePath, setActivePath] = useState(resources[0]?.path ?? "");
  const [editing, setEditing] = useState<Editing | null>(null);

  if (!auth.token) return <Login onLogin={auth.login} />;

  const current = resources.find((r) => r.path === activePath) ?? resources[0];

  return (
    <QueryClientProvider client={client}>
      <div className="app">
        <header className="topbar">
          <div className="brand">
            <span className="mark">g</span>
          </div>
          <nav className="crumbs" aria-label="Breadcrumb">
            <span>Home</span>
            <span className="sep">/</span>
            <span className="cur">{current ? titleCase(current.path) : ""}</span>
          </nav>
          <span className="spacer" />
          <button
            className="iconbtn"
            title="Toggle theme"
            aria-label="Toggle theme"
            onClick={() => setThemeState(toggleTheme())}
          >
            <Icon name={theme === "dark" ? "moon" : "sun"} />
          </button>
          <div className="user">
            <span className="avatar">
              <Icon name="user" />
            </span>
            <span className="who">
              <b>Account</b>
              <small>Signed in</small>
            </span>
          </div>
          <button className="iconbtn" title="Sign out" aria-label="Sign out" onClick={auth.logout}>
            <Icon name="logout" />
          </button>
        </header>

        <div className="shell">
          <aside className="side">
            <div className="group">Resources</div>
            <nav className="nav">
              {resources.map((r) => (
                <button
                  key={r.path}
                  className={r.path === current?.path ? "active" : ""}
                  onClick={() => setActivePath(r.path)}
                >
                  <Icon name={r.path === "users" ? "user" : "doc"} />
                  <span>{titleCase(r.path)}</span>
                </button>
              ))}
            </nav>
            <div className="foot">
              <span>admin</span>
              <span className="mono">Gize</span>
            </div>
          </aside>

          <main className="main">
            {current && (
              <Resource
                key={current.path}
                desc={current}
                onAdd={() => setEditing({ desc: current, row: null })}
                onEdit={(row) => setEditing({ desc: current, row })}
              />
            )}
          </main>
        </div>

        <div className={"scrim" + (editing ? " show" : "")} onClick={() => setEditing(null)} />
        {editing && (
          <ResourceForm
            desc={editing.desc}
            row={editing.row}
            onClose={() => setEditing(null)}
          />
        )}
      </div>
      <ToastHost />
    </QueryClientProvider>
  );
}
