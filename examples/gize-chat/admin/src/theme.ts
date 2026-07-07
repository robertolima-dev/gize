export type Theme = "light" | "dark";

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
