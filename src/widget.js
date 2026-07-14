const { invoke } = window.__TAURI__.core;
const { listen, emit } = window.__TAURI__.event;
const { getCurrentWindow, LogicalPosition } = window.__TAURI__.window;
const { Menu, MenuItem, CheckMenuItem } = window.__TAURI__.menu;

const card = document.getElementById("card");
const langEl = document.getElementById("lang");
const pinBtn = document.getElementById("pin");
const lockBtn = document.getElementById("lock");
const gearBtn = document.getElementById("gear");
const customCssEl = document.getElementById("custom-css");

let settings = null;

// ---------- Appearance ----------
function hexToRgba(hex, opacityPct) {
  hex = hex.replace("#", "");
  const r = parseInt(hex.slice(0, 2), 16);
  const g = parseInt(hex.slice(2, 4), 16);
  const b = parseInt(hex.slice(4, 6), 16);
  const a = Math.max(0, Math.min(100, opacityPct)) / 100;
  return `rgba(${r}, ${g}, ${b}, ${a})`;
}

function applyAppearance(s) {
  const base = 16 * (s.scale || 1);
  card.style.fontSize = base + "px";
  card.style.borderRadius = (s.radius || 0) * (s.scale || 1) + "px";
  card.style.background = hexToRgba(s.bg_color || "1E1E1E", s.opacity ?? 100);
  card.dataset.theme = s.theme || "default";
  card.classList.toggle("pinned", !!s.pos_locked);
  updateDragRegion(!!s.pos_locked);
  lockBtn.style.display = (s.show_lock !== false) ? "flex" : "none";
  if (document.querySelector(".stack")) {
    const stack = document.querySelector(".stack");
    stack.style.display = ((s.show_pin !== false) || (s.show_settings !== false)) ? "flex" : "none";
    pinBtn.style.display = (s.show_pin !== false) ? "flex" : "none";
    gearBtn.style.display = (s.show_settings !== false) ? "flex" : "none";
  }
  customCssEl.textContent = s.custom_css || "";
  requestAnimationFrame(fitWindow);
}

function fitWindow() {
  const rect = card.getBoundingClientRect();
  const w = Math.ceil(rect.width) + 4;
  const h = Math.ceil(rect.height) + 4;
  invoke("resize_widget", { width: w, height: h });
}

// Drag works on the card *and* the language label / main row (buttons are
// excluded so their clicks still register). Pinning disables dragging on all.
const DRAG_ELS = [card, langEl, document.querySelector(".main")];
function updateDragRegion(pinned) {
  DRAG_ELS.forEach((el) => {
    if (!el) return;
    if (pinned) el.removeAttribute("data-tauri-drag-region");
    else el.setAttribute("data-tauri-drag-region", "");
  });
}

// ---------- Status ----------
function applyStatus(lang, locked) {
  langEl.textContent = lang;
  card.classList.toggle("locked", !!locked);
}

// ---------- Context menu ----------
card.addEventListener("contextmenu", async (e) => {
  e.preventDefault();
  e.stopPropagation();
  invoke("open_settings");
});

// ---------- Actions ----------
lockBtn.addEventListener("click", async (e) => {
  e.stopPropagation();
  const locked = await invoke("toggle_lock");
  card.classList.toggle("locked", locked);
  if (settings) { settings.layout_locked = locked; persist(); }
});

pinBtn.addEventListener("click", (e) => {
  e.stopPropagation();
  if (!settings) return;
  settings.pos_locked = !settings.pos_locked;
  card.classList.toggle("pinned", settings.pos_locked);
  updateDragRegion(settings.pos_locked);
  persist();
});

gearBtn.addEventListener("click", (e) => {
  e.stopPropagation();
  invoke("open_settings");
});

function persist() {
  if (settings) invoke("save_settings", { settings });
}

// ---------- Position persistence ----------
let moveTimer = null;
getCurrentWindow().onMoved(({ payload }) => {
  if (!settings) return;
  clearTimeout(moveTimer);
  moveTimer = setTimeout(async () => {
    const factor = await getCurrentWindow().scaleFactor();
    settings.pos_x = Math.round(payload.x / factor);
    settings.pos_y = Math.round(payload.y / factor);
    persist();
  }, 400);
});

// ---------- Live updates ----------
listen("layout", (e) => applyStatus(e.payload.lang, e.payload.locked));
listen("settings-changed", (e) => {
  settings = e.payload;
  applyAppearance(settings);
});

// ---------- Init ----------
(async () => {
  settings = await invoke("get_settings");
  applyAppearance(settings);
  const status = await invoke("get_status");
  applyStatus(status.lang, status.locked);
})();
