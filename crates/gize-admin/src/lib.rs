//! Admin UI generator for Gize (ADR-006).
//!
//! `gize make admin` generates a **separate** Vite + React + TypeScript SPA under `admin/`,
//! driven by the manifest. The app is data-driven: [`resources_ts`] emits one descriptor
//! (fields, relationships and a Zod schema) per resource from `gize.toml`, and generic
//! components render List/Create/Edit/Delete for any of them. It talks to the backend through
//! a Vite dev proxy (`/api` → the API), so no CORS or backend changes are needed.
//!
//! The UI is a token-based design system (light + dark theme) with a Django-inspired layout:
//! an app shell, a changelist with search/filters/pagination/bulk actions, and a slide-over
//! drawer form. Templates are Rust functions returning file contents.

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
    "@types/react": "^18.3.11",
    "@types/react-dom": "^18.3.1",
    "@vitejs/plugin-react": "^4.3.2",
    "typescript": "^5.6.2",
    "vite": "^5.4.8"
  }},
  "pnpm": {{
    "onlyBuiltDependencies": ["esbuild"]
  }}
}}
"#
    )
}

/// `admin/vite.config.ts` — React, and a dev proxy so `/api/*` reaches the backend.
pub fn vite_config() -> String {
    r#"import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The API base is proxied so the browser talks same-origin in dev. Set VITE_API_URL to point
// at your running Gize backend (default http://localhost:8080).
export default defineConfig({
  plugins: [react()],
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

/// `admin/index.html`. Sets the theme before first paint to avoid a flash.
pub fn index_html() -> String {
    r####"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Admin</title>
    <script>
      try {
        var t = localStorage.getItem("gize-admin-theme");
        if (t === "dark" || t === "light") document.documentElement.setAttribute("data-theme", t);
      } catch (e) {}
    </script>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
"####
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

/// `admin/src/styles.css` — the token-based design system (light + dark).
pub fn styles_css() -> String {
    include_str!("assets/styles.css").to_string()
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

/// `admin/src/theme.ts` — light/dark theme helpers (persisted, system-aware).
pub fn theme_ts() -> String {
    r#"export type Theme = "light" | "dark";

export function currentTheme(): Theme {
  const stored = safeGet();
  if (stored === "light" || stored === "dark") return stored;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export function setTheme(theme: Theme): void {
  document.documentElement.setAttribute("data-theme", theme);
  try {
    localStorage.setItem("gize-admin-theme", theme);
  } catch {
    /* ignore */
  }
}

export function toggleTheme(): Theme {
  const next: Theme = currentTheme() === "dark" ? "light" : "dark";
  setTheme(next);
  return next;
}

function safeGet(): string | null {
  try {
    return localStorage.getItem("gize-admin-theme");
  } catch {
    return null;
  }
}
"#
    .to_string()
}

/// `admin/src/icons.tsx` — a small inline SVG icon set.
pub fn icons_tsx() -> String {
    include_str!("assets/icons.tsx").to_string()
}

/// `admin/src/toast.tsx` — a lightweight toast (event-based, no context wiring).
pub fn toast_tsx() -> String {
    r#"import { useEffect, useState } from "react";
import { Icon } from "./icons";

export function toast(message: string): void {
  window.dispatchEvent(new CustomEvent("gize-toast", { detail: message }));
}

export function ToastHost() {
  const [msg, setMsg] = useState<string | null>(null);
  useEffect(() => {
    let timer: number | undefined;
    function onToast(e: Event) {
      setMsg((e as CustomEvent<string>).detail);
      window.clearTimeout(timer);
      timer = window.setTimeout(() => setMsg(null), 2400);
    }
    window.addEventListener("gize-toast", onToast);
    return () => {
      window.removeEventListener("gize-toast", onToast);
      window.clearTimeout(timer);
    };
  }, []);
  return (
    <div className={"toast" + (msg ? " show" : "")} role="status" aria-live="polite">
      <Icon name="check" className="ok" />
      <span>{msg}</span>
    </div>
  );
}
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
  list: (path: string): Promise<any[]> => req(`/${path}`),
  get: (path: string, id: string) => req(`/${path}/${id}`),
  create: (path: string, body: unknown) =>
    req(`/${path}`, { method: "POST", body: JSON.stringify(body) }),
  update: (path: string, id: string, body: unknown) =>
    req(`/${path}/${id}`, { method: "PUT", body: JSON.stringify(body) }),
  remove: (path: string, id: string) => req(`/${path}/${id}`, { method: "DELETE" }),
  login: (email: string, password: string) =>
    req(`/users/login`, { method: "POST", body: JSON.stringify({ email, password }) }),
};

/** A resource's display label: the (plural) path, capitalized — e.g. "posts" -> "Posts". */
export function titleCase(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

/** A readable label for a related record (used by foreign-key selects and cells). */
export function labelOf(row: Record<string, any>): string {
  for (const key of ["name", "title", "email", "slug"]) {
    if (row[key]) return String(row[key]);
  }
  for (const key of Object.keys(row)) {
    const value = row[key];
    if (typeof value === "string" && key !== "id" && !key.endsWith("_at")) return value;
  }
  return String(row.id ?? "");
}
"#
    .to_string()
}

/// `admin/src/auth.tsx` — token storage hook and the branded login screen.
pub fn auth_tsx() -> String {
    include_str!("assets/auth.tsx").to_string()
}

/// `admin/src/App.tsx` — the app shell (topbar, sidebar, drawer, toasts, theme toggle).
pub fn app_tsx() -> String {
    include_str!("assets/App.tsx").to_string()
}

/// `admin/src/Resource.tsx` — the changelist (search, filters, pagination, bulk actions).
pub fn resource_tsx() -> String {
    include_str!("assets/Resource.tsx").to_string()
}

/// `admin/src/ResourceForm.tsx` — the drawer form (fieldsets, typed inputs, FK selects).
pub fn resource_form_tsx() -> String {
    include_str!("assets/ResourceForm.tsx").to_string()
}

/// `admin/src/resources.ts` — the descriptors generated from the manifest (ADR-006): one entry
/// per resource with its editable fields (each field carrying its kind, and a `ref` target for
/// foreign keys) and a Zod schema mirroring the backend validation.
pub fn resources_ts(manifest: &Manifest) -> anyhow::Result<String> {
    let mut entries = String::new();
    for module in &manifest.modules {
        let model = module.model_spec()?;
        let table = table_name(&model.name);

        let mut fields_ts = String::new();
        let mut schema_ts = String::new();
        for f in &model.fields {
            let kind = field_kind(&f.name, f.ty);
            // A synthetic `<name>_id` foreign-key column references `belongs_to` target's table.
            let ref_target = model
                .relations
                .iter()
                .find(|r| r.fk_column() == f.name)
                .map(|r| r.target.clone());
            let ref_ts = match &ref_target {
                Some(t) => format!(", ref: \"{t}\""),
                None => String::new(),
            };
            fields_ts.push_str(&format!(
                "      {{ name: \"{}\", kind: \"{kind}\"{ref_ts} }},\n",
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
  /** For a foreign key: the target resource path (e.g. "users"). */
  ref?: string;
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
        assert!(ts.contains("{ name: \"email\", kind: \"email\" }"));
        assert!(ts.contains("email: z.string().email()"));
        assert!(ts.contains("password: z.string().min(8)"));
        // the FK column is a uuid field carrying its ref target.
        assert!(ts.contains("{ name: \"author_id\", kind: \"uuid\", ref: \"users\" }"));
    }

    #[test]
    fn package_json_uses_the_project_name() {
        assert!(package_json("blog").contains("\"name\": \"blog-admin\""));
    }

    #[test]
    fn package_json_approves_esbuild_build_so_serve_works_without_pnpm_approve_builds() {
        // pnpm v10 blocks dependency build scripts by default; whitelisting esbuild lets the
        // first-run `pnpm install` build it, so `gize serve` works without `pnpm approve-builds`.
        assert!(package_json("blog").contains("\"onlyBuiltDependencies\": [\"esbuild\"]"));
    }
}
