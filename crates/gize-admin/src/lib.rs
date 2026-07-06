//! Admin UI generator for Gize (ADR-006).
//!
//! `gize make admin` generates a **separate** Vite + React + TypeScript SPA under `admin/`,
//! driven by the manifest. The app is data-driven: [`resources_ts`] emits one descriptor
//! (fields + a Zod schema) per resource from `gize.toml`, and a single generic `Resource`
//! component renders List/Create/Edit/Delete for any of them. It talks to the backend through
//! a Vite dev proxy (`/api` → the API), so no CORS or backend changes are needed.
//!
//! Templates are Rust functions returning file contents (like `gize-templates`); the
//! generator (`gize-generator`) assembles them into a `Plan`.

use gize_core::naming::table_name;
use gize_core::{FieldType, Manifest};

/// `admin/package.json` — the SPA's dependencies and scripts.
pub fn package_json(project: &str) -> String {
    format!(
        r#"{{
  "name": "{project}-admin",
  "private": true,
  "type": "module",
  "scripts": {{
    "dev": "vite",
    "build": "tsc --noEmit && vite build",
    "preview": "vite preview"
  }},
  "dependencies": {{
    "@hookform/resolvers": "^3.9.0",
    "@tanstack/react-query": "^5.59.0",
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "react-hook-form": "^7.53.0",
    "zod": "^3.23.8"
  }},
  "devDependencies": {{
    "@tailwindcss/vite": "^4.0.0",
    "@types/react": "^18.3.11",
    "@types/react-dom": "^18.3.1",
    "@vitejs/plugin-react": "^4.3.2",
    "tailwindcss": "^4.0.0",
    "typescript": "^5.6.2",
    "vite": "^5.4.8"
  }}
}}
"#
    )
}

/// `admin/vite.config.ts` — React + Tailwind, and a dev proxy so `/api/*` reaches the backend
/// (avoids CORS in development; ADR-006).
pub fn vite_config() -> String {
    r#"import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// The API base is proxied so the browser talks same-origin in dev. Set VITE_API_URL to point
// at your running Gize backend (default http://localhost:8080).
export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    proxy: {
      "/api": {
        target: process.env.VITE_API_URL || "http://localhost:8080",
        changeOrigin: true,
        rewrite: (p) => p.replace(/^\/api/, ""),
      },
    },
  },
});
"#
    .to_string()
}

/// `admin/tsconfig.json`.
pub fn tsconfig() -> String {
    r#"{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": false,
    "noUnusedParameters": false,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"]
}
"#
    .to_string()
}

/// `admin/index.html`.
pub fn index_html() -> String {
    r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Admin</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
"#
    .to_string()
}

/// `admin/.env.example`.
pub fn env_example() -> String {
    "# Point at your running Gize backend (proxied under /api in dev).\nVITE_API_URL=http://localhost:8080\n"
        .to_string()
}

/// `admin/.gitignore`.
pub fn gitignore() -> String {
    "node_modules\ndist\n.env\n".to_string()
}

/// `admin/src/styles.css` — Tailwind v4 entry.
pub fn styles_css() -> String {
    "@import \"tailwindcss\";\n".to_string()
}

/// `admin/src/main.tsx`.
pub fn main_tsx() -> String {
    r#"import React from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles.css";

createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
"#
    .to_string()
}

/// `admin/src/api.ts` — a tiny typed fetch client that attaches the JWT and hits `/api`.
pub fn api_ts() -> String {
    r#"const BASE = "/api";

export function getToken(): string | null {
  return localStorage.getItem("gize_token");
}

async function req(path: string, opts: RequestInit = {}): Promise<any> {
  const headers: Record<string, string> = {
    "content-type": "application/json",
    ...(opts.headers as Record<string, string>),
  };
  const t = getToken();
  if (t) headers["authorization"] = `Bearer ${t}`;
  const res = await fetch(`${BASE}${path}`, { ...opts, headers });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(body || String(res.status));
  }
  if (res.status === 204) return null;
  return res.json();
}

export const api = {
  list: (path: string) => req(`/${path}`),
  get: (path: string, id: string) => req(`/${path}/${id}`),
  create: (path: string, body: unknown) =>
    req(`/${path}`, { method: "POST", body: JSON.stringify(body) }),
  update: (path: string, id: string, body: unknown) =>
    req(`/${path}/${id}`, { method: "PUT", body: JSON.stringify(body) }),
  remove: (path: string, id: string) => req(`/${path}/${id}`, { method: "DELETE" }),
  login: (email: string, password: string) =>
    req(`/users/login`, { method: "POST", body: JSON.stringify({ email, password }) }),
};
"#
    .to_string()
}

/// `admin/src/auth.tsx` — token storage hook and a login screen.
pub fn auth_tsx() -> String {
    r#"import { useState } from "react";
import { api } from "./api";

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
  const [error, setError] = useState("");

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError("");
    try {
      const res = await api.login(email, password);
      onLogin(res.token);
    } catch {
      setError("Invalid credentials");
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50">
      <form onSubmit={submit} className="bg-white p-8 rounded-lg shadow w-80 space-y-4">
        <h1 className="text-xl font-bold">Sign in</h1>
        <input
          className="w-full border rounded px-3 py-2"
          placeholder="Email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
        />
        <input
          className="w-full border rounded px-3 py-2"
          type="password"
          placeholder="Password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
        />
        {error && <p className="text-red-600 text-sm">{error}</p>}
        <button className="w-full bg-black text-white rounded py-2" type="submit">
          Sign in
        </button>
      </form>
    </div>
  );
}
"#
    .to_string()
}

/// `admin/src/App.tsx` — the shell: auth gate, a sidebar of resources, and the active screen.
pub fn app_tsx() -> String {
    r#"import { useState } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { resources } from "./resources";
import { Resource } from "./Resource";
import { Login, useAuth } from "./auth";

const client = new QueryClient();

export default function App() {
  const auth = useAuth();
  const [active, setActive] = useState(resources[0]?.path ?? "");

  if (!auth.token) return <Login onLogin={auth.login} />;

  const current = resources.find((r) => r.path === active) ?? resources[0];

  return (
    <QueryClientProvider client={client}>
      <div className="min-h-screen flex bg-gray-50 text-gray-900">
        <aside className="w-56 bg-white border-r p-4 flex flex-col gap-1">
          <div className="font-bold mb-3">Admin</div>
          {resources.map((r) => (
            <button
              key={r.path}
              onClick={() => setActive(r.path)}
              className={`text-left px-3 py-2 rounded ${
                current && r.path === current.path ? "bg-gray-900 text-white" : "hover:bg-gray-100"
              }`}
            >
              {r.name}
            </button>
          ))}
          <button onClick={auth.logout} className="mt-auto text-sm text-gray-500 px-3 py-2">
            Sign out
          </button>
        </aside>
        <main className="flex-1 p-8">
          {current && <Resource key={current.path} desc={current} />}
        </main>
      </div>
    </QueryClientProvider>
  );
}
"#
    .to_string()
}

/// `admin/src/Resource.tsx` — the generic CRUD screen (list + search + pagination + a form for
/// create/edit + delete), driven by a resource descriptor.
pub fn resource_tsx() -> String {
    r#"import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { api } from "./api";
import type { ResourceDesc } from "./resources";

type Row = Record<string, any>;

export function Resource({ desc }: { desc: ResourceDesc }) {
  const qc = useQueryClient();
  const [editing, setEditing] = useState<Row | null>(null);
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(0);
  const pageSize = 10;

  const list = useQuery<Row[]>({ queryKey: [desc.path], queryFn: () => api.list(desc.path) });
  const form = useForm<Row>({ resolver: zodResolver(desc.createSchema) as any });

  const save = useMutation({
    mutationFn: (values: Row) =>
      editing ? api.update(desc.path, editing.id, values) : api.create(desc.path, values),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: [desc.path] });
      form.reset({});
      setEditing(null);
    },
  });

  const remove = useMutation({
    mutationFn: (id: string) => api.remove(desc.path, id),
    onSuccess: () => qc.invalidateQueries({ queryKey: [desc.path] }),
  });

  const items = list.data ?? [];
  const filtered = items.filter((it) =>
    JSON.stringify(it).toLowerCase().includes(search.toLowerCase()),
  );
  const pages = Math.max(1, Math.ceil(filtered.length / pageSize));
  const shown = filtered.slice(page * pageSize, page * pageSize + pageSize);
  const columns = desc.fields.filter((f) => f.kind !== "password");

  function startEdit(item: Row) {
    setEditing(item);
    const values: Row = {};
    for (const f of desc.fields) values[f.name] = item[f.name] ?? "";
    form.reset(values);
  }
  function startCreate() {
    setEditing(null);
    form.reset({});
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-3">
        <h1 className="text-2xl font-bold">{desc.name}</h1>
        <input
          className="border rounded px-3 py-1 ml-auto"
          placeholder="Search"
          value={search}
          onChange={(e) => {
            setSearch(e.target.value);
            setPage(0);
          }}
        />
        <button className="bg-gray-900 text-white rounded px-3 py-1" onClick={startCreate}>
          New
        </button>
      </div>

      <form
        onSubmit={form.handleSubmit((v) => save.mutate(v))}
        className="bg-white rounded-lg shadow p-4 grid grid-cols-2 gap-3"
        data-testid="resource-form"
      >
        <div className="col-span-2 font-semibold">{editing ? "Edit" : "Create"}</div>
        {desc.fields.map((f) => (
          <label key={f.name} className="text-sm flex flex-col gap-1">
            {f.name}
            {f.kind === "boolean" ? (
              <input type="checkbox" {...form.register(f.name)} />
            ) : (
              <input
                className="border rounded px-2 py-1"
                type={f.kind === "number" ? "number" : f.kind === "password" ? "password" : "text"}
                {...form.register(f.name)}
              />
            )}
            {form.formState.errors[f.name] && (
              <span className="text-red-600 text-xs">
                {String(form.formState.errors[f.name]?.message ?? "invalid")}
              </span>
            )}
          </label>
        ))}
        <div className="col-span-2 flex gap-2">
          <button className="bg-black text-white rounded px-4 py-1" type="submit">
            {editing ? "Save" : "Create"}
          </button>
          {editing && (
            <button type="button" className="px-4 py-1" onClick={startCreate}>
              Cancel
            </button>
          )}
          {save.isError && (
            <span className="text-red-600 text-sm self-center">{String(save.error)}</span>
          )}
        </div>
      </form>

      <div className="bg-white rounded-lg shadow overflow-x-auto">
        <table className="w-full text-sm">
          <thead className="bg-gray-100 text-left">
            <tr>
              {columns.map((c) => (
                <th key={c.name} className="px-3 py-2">
                  {c.name}
                </th>
              ))}
              <th className="px-3 py-2" />
            </tr>
          </thead>
          <tbody>
            {shown.map((item) => (
              <tr key={item.id} className="border-t" data-testid="row">
                {columns.map((c) => (
                  <td key={c.name} className="px-3 py-2">
                    {String(item[c.name] ?? "")}
                  </td>
                ))}
                <td className="px-3 py-2 text-right whitespace-nowrap">
                  <button className="text-blue-600 mr-3" onClick={() => startEdit(item)}>
                    Edit
                  </button>
                  <button className="text-red-600" onClick={() => remove.mutate(item.id)}>
                    Delete
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="flex items-center gap-3 text-sm">
        <button
          disabled={page === 0}
          onClick={() => setPage(page - 1)}
          className="px-2 py-1 border rounded disabled:opacity-40"
        >
          Prev
        </button>
        <span>
          Page {page + 1} / {pages}
        </span>
        <button
          disabled={page + 1 >= pages}
          onClick={() => setPage(page + 1)}
          className="px-2 py-1 border rounded disabled:opacity-40"
        >
          Next
        </button>
      </div>
    </div>
  );
}
"#
    .to_string()
}

/// `admin/src/resources.ts` — the descriptors generated from the manifest (ADR-006): one entry
/// per resource with its editable fields and a Zod schema mirroring the backend validation.
pub fn resources_ts(manifest: &Manifest) -> anyhow::Result<String> {
    let mut entries = String::new();
    for module in &manifest.modules {
        let model = module.model_spec()?;
        let table = table_name(&model.name);

        let mut fields_ts = String::new();
        let mut schema_ts = String::new();
        for f in &model.fields {
            let kind = field_kind(&f.name, f.ty);
            fields_ts.push_str(&format!(
                "      {{ name: \"{}\", kind: \"{kind}\" }},\n",
                f.name
            ));
            schema_ts.push_str(&format!("      {}: {},\n", f.name, zod_for(kind)));
        }

        entries.push_str(&format!(
            "  {{\n    name: \"{name}\",\n    path: \"{table}\",\n    fields: [\n{fields_ts}    ],\n    createSchema: z.object({{\n{schema_ts}    }}),\n  }},\n",
            name = model.name,
        ));
    }

    Ok(format!(
        r#"import {{ z }} from "zod";

export type FieldKind =
  | "string"
  | "number"
  | "boolean"
  | "uuid"
  | "datetime"
  | "email"
  | "password";

export interface FieldDesc {{
  name: string;
  kind: FieldKind;
}}

export interface ResourceDesc {{
  name: string;
  path: string;
  fields: FieldDesc[];
  createSchema: z.ZodTypeAny;
}}

export const resources: ResourceDesc[] = [
{entries}];
"#
    ))
}

/// The admin field kind for a model field: `email`/`password` are recognized by name (mirroring
/// the backend's users validation), everything else maps from the scalar type.
fn field_kind(name: &str, ty: FieldType) -> &'static str {
    match name {
        "email" => "email",
        "password" => "password",
        _ => match ty {
            FieldType::String => "string",
            FieldType::Bool => "boolean",
            FieldType::I32 | FieldType::I64 | FieldType::F64 => "number",
            FieldType::Uuid => "uuid",
            FieldType::DateTime => "datetime",
        },
    }
}

/// The Zod validator for a field kind, mirroring the backend `validator` rules.
fn zod_for(kind: &str) -> &'static str {
    match kind {
        "email" => "z.string().email()",
        "password" => "z.string().min(8)",
        "number" => "z.coerce.number()",
        "boolean" => "z.boolean()",
        "uuid" => "z.string().uuid()",
        "datetime" => "z.string().min(1)",
        _ => "z.string().min(1)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gize_core::{Module, Relation};

    fn manifest() -> Manifest {
        let mut m = Manifest::new("blog");
        m.upsert_module(Module {
            name: "users".to_string(),
            fields: vec![
                "name:String".to_string(),
                "email:String".to_string(),
                "password:String".to_string(),
                "is_admin:bool".to_string(),
            ],
            belongs_to: vec![],
        });
        m.upsert_module(Module {
            name: "posts".to_string(),
            fields: vec!["title:String".to_string()],
            belongs_to: vec![Relation {
                field: "author".to_string(),
                target: "users".to_string(),
            }],
        });
        m
    }

    #[test]
    fn resources_ts_describes_each_resource() {
        let ts = resources_ts(&manifest()).unwrap();
        assert!(ts.contains("path: \"users\""));
        assert!(ts.contains("path: \"posts\""));
        // email/password get the specialized kinds + Zod rules.
        assert!(ts.contains("{ name: \"email\", kind: \"email\" }"));
        assert!(ts.contains("email: z.string().email()"));
        assert!(ts.contains("password: z.string().min(8)"));
        // the FK column is present as a uuid field.
        assert!(ts.contains("{ name: \"author_id\", kind: \"uuid\" }"));
    }

    #[test]
    fn package_json_uses_the_project_name() {
        assert!(package_json("blog").contains("\"name\": \"blog-admin\""));
    }
}
