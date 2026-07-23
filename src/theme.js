import { STORAGE_KEY, THEME_KEY, LEGACY_STORAGE_KEY, LEGACY_THEME_KEY } from "./constants.js";

/** One-time migrate localStorage from older private builds */
export function migrateLegacyStorage() {
  try {
    if (!localStorage.getItem(STORAGE_KEY) && localStorage.getItem(LEGACY_STORAGE_KEY)) {
      localStorage.setItem(STORAGE_KEY, localStorage.getItem(LEGACY_STORAGE_KEY));
    }
    if (!localStorage.getItem(THEME_KEY) && localStorage.getItem(LEGACY_THEME_KEY)) {
      localStorage.setItem(THEME_KEY, localStorage.getItem(LEGACY_THEME_KEY));
    }
  } catch {
    /* ignore */
  }
}

export function getTheme() {
  const t = document.documentElement.getAttribute("data-theme");
  return t === "dark" ? "dark" : "light";
}

export function setTheme(theme) {
  const next = theme === "dark" ? "dark" : "light";
  document.documentElement.setAttribute("data-theme", next);
  try {
    localStorage.setItem(THEME_KEY, next);
  } catch {
    /* ignore */
  }
  const btn = document.getElementById("btn-theme");
  if (btn) {
    btn.title = next === "light" ? "Switch to dark theme" : "Switch to light theme";
    btn.setAttribute(
      "aria-label",
      next === "light" ? "Switch to dark theme" : "Switch to light theme"
    );
  }
  const themeSeg = document.getElementById("theme-options");
  if (themeSeg) {
    themeSeg.querySelectorAll("button[data-value]").forEach((b) => {
      b.classList.toggle("active", b.dataset.value === next);
    });
  }
}

export function toggleTheme() {
  setTheme(getTheme() === "light" ? "dark" : "light");
}

export function initTheme() {
  let theme = "light";
  try {
    const saved = localStorage.getItem(THEME_KEY);
    if (saved === "light" || saved === "dark") theme = saved;
    else if (window.matchMedia("(prefers-color-scheme: dark)").matches) theme = "dark";
  } catch {
    theme = "light";
  }
  setTheme(theme);

  document.getElementById("btn-theme")?.addEventListener("click", () => {
    toggleTheme();
  });

  document.getElementById("theme-options")?.addEventListener("click", (e) => {
    const btn = e.target.closest("button[data-value]");
    if (!btn) return;
    setTheme(btn.dataset.value);
  });
}
