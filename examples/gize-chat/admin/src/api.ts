const BASE = "/api";

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
