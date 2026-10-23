const BASE = "/api";

export function getToken(): string | null {
  return localStorage.getItem("gize_token");
}

/** A failed API call. `status` tells a refused request (401/403) apart from a broken one. */
export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

/** The HTTP status of a failed call, or undefined when the request never got an answer. */
export function statusOf(error: unknown): number | undefined {
  return error instanceof ApiError ? error.status : undefined;
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
    throw new ApiError(res.status, body || String(res.status));
  }
  if (res.status === 204) return null;
  return res.json();
}

/** Largest page the API serves (ADR-022). */
const PER_PAGE = 100;

/** Every row of a resource: pages of PER_PAGE until the last one, so search, filters and
 * foreign-key selects can keep working on the full list in the browser. */
async function listAll(path: string): Promise<any[]> {
  const rows: any[] = [];
  for (let page = 1; ; page++) {
    const res = await req(`/${path}?page=${page}&per_page=${PER_PAGE}`);
    rows.push(...res.items);
    if (res.items.length < PER_PAGE || rows.length >= res.total) return rows;
  }
}

export const api = {
  list: listAll,
  get: (path: string, id: string) => req(`/${path}/${id}`),
  create: (path: string, body: unknown) =>
    req(`/${path}`, { method: "POST", body: JSON.stringify(body) }),
  update: (path: string, id: string, body: unknown) =>
    req(`/${path}/${id}`, { method: "PUT", body: JSON.stringify(body) }),
  remove: (path: string, id: string) => req(`/${path}/${id}`, { method: "DELETE" }),
  login: (email: string, password: string) =>
    req(`/users/login`, { method: "POST", body: JSON.stringify({ email, password }) }),
};

/** A resource's display label: the (plural) path in words, capitalized — e.g.
 * "posts" -> "Posts", "ticket_types" -> "Ticket types". */
export function titleCase(s: string): string {
  const words = s.replace(/_/g, " ");
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/** A model name as a lowercase noun for buttons and dialogs — e.g. "TicketType" -> "ticket type". */
export function nounOf(model: string): string {
  return model.replace(/([a-z0-9])([A-Z])/g, "$1 $2").toLowerCase();
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
