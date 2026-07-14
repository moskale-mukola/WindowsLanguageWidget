const { invoke } = window.__TAURI__.core;
const { emit, listen } = window.__TAURI__.event;

const PRESETS = ["1E1E1E", "000000", "2D2D30", "0F0F0F", "1A1A2E", "16213E", "0D1117", "3C3C3C"];

// Baseline appearance applied when a theme is picked; sliders can still be
// adjusted afterward on top of it.
const THEME_PRESETS = {
  "default": { bg_color: "1E1E1E", radius: 16, opacity: 100 },
  "win11-light": { bg_color: "F3F3F3", radius: 10, opacity: 95 },
  "win11-dark": { bg_color: "202020", radius: 10, opacity: 95 },
  "minimal": { bg_color: "000000", radius: 6, opacity: 55 },
  "futuristic": { bg_color: "0D0D14", radius: 8, opacity: 90 },
};

const els = {
  uiLang: document.getElementById("uiLang"),
  theme: document.getElementById("theme"),
  size: document.getElementById("size"),
  sizeVal: document.getElementById("sizeVal"),
  opacity: document.getElementById("opacity"),
  opacityVal: document.getElementById("opacityVal"),
  radius: document.getElementById("radius"),
  radiusVal: document.getElementById("radiusVal"),
  swatches: document.getElementById("swatches"),
  colorPicker: document.getElementById("colorPicker"),
  hex: document.getElementById("hex"),
  aot: document.getElementById("aot"),
  autostart: document.getElementById("autostart"),
  blockHotkeys: document.getElementById("blockHotkeys"),
  showLock: document.getElementById("showLock"),
  showPin: document.getElementById("showPin"),
  showSettings: document.getElementById("showSettings"),
  customCss: document.getElementById("customCss"),
  resetBtn: document.getElementById("resetBtn"),
  checkUpdate: document.getElementById("checkUpdate"),
  exitBtn: document.getElementById("exitBtn"),
};

let settings = null;

function normHex(v) {
  v = v.replace("#", "").trim();
  return /^[0-9a-fA-F]{6}$/.test(v) ? v.toUpperCase() : null;
}

function buildSwatches() {
  PRESETS.forEach((hex) => {
    const d = document.createElement("div");
    d.className = "swatch";
    d.style.background = "#" + hex;
    d.dataset.hex = hex;
    d.addEventListener("click", () => setColor(hex));
    els.swatches.appendChild(d);
  });
}

function markActiveSwatch(hex) {
  [...els.swatches.children].forEach((s) =>
    s.classList.toggle("active", s.dataset.hex.toUpperCase() === hex.toUpperCase())
  );
}

function apply() {
  invoke("save_settings", { settings });
  emit("settings-changed", settings);
}

function setColor(hex) {
  const n = normHex(hex);
  if (!n) return;
  settings.bg_color = n;
  els.hex.value = "#" + n;
  els.colorPicker.value = "#" + n;
  markActiveSwatch(n);
  apply();
}

// Fills every control from the current `settings` object.
function populateUI() {
  els.theme.value = settings.theme || "default";
  els.size.value = Math.round(settings.scale * 100);
  els.sizeVal.textContent = els.size.value + "%";
  els.opacity.value = settings.opacity;
  els.opacityVal.textContent = settings.opacity + "%";
  els.radius.value = settings.radius;
  els.radiusVal.textContent = settings.radius + "px";
  els.hex.value = "#" + settings.bg_color;
  els.colorPicker.value = "#" + settings.bg_color;
  markActiveSwatch(settings.bg_color);
  els.aot.checked = settings.always_on_top;
  els.blockHotkeys.checked = settings.block_hotkeys === true;
  els.showLock.checked = settings.show_lock !== false;
  els.showPin.checked = settings.show_pin !== false;
  els.showSettings.checked = settings.show_settings !== false;
  els.customCss.value = settings.custom_css || "";
}

// ---------- Wiring ----------
els.theme.addEventListener("change", () => {
  settings.theme = els.theme.value;
  const preset = THEME_PRESETS[settings.theme];
  if (preset) {
    settings.bg_color = preset.bg_color;
    settings.radius = preset.radius;
    settings.opacity = preset.opacity;
    els.radius.value = preset.radius;
    els.radiusVal.textContent = preset.radius + "px";
    els.opacity.value = preset.opacity;
    els.opacityVal.textContent = preset.opacity + "%";
    els.hex.value = "#" + preset.bg_color;
    els.colorPicker.value = "#" + preset.bg_color;
    markActiveSwatch(preset.bg_color);
  }
  apply();
});

els.size.addEventListener("input", () => {
  settings.scale = els.size.value / 100;
  els.sizeVal.textContent = els.size.value + "%";
  apply();
});

els.opacity.addEventListener("input", () => {
  settings.opacity = +els.opacity.value;
  els.opacityVal.textContent = els.opacity.value + "%";
  apply();
});

els.radius.addEventListener("input", () => {
  settings.radius = +els.radius.value;
  els.radiusVal.textContent = els.radius.value + "px";
  apply();
});

els.colorPicker.addEventListener("input", () => setColor(els.colorPicker.value));
els.hex.addEventListener("change", () => setColor(els.hex.value));

els.aot.addEventListener("change", () => {
  settings.always_on_top = els.aot.checked;
  invoke("set_always_on_top", { value: settings.always_on_top });
  apply();
});

els.autostart.addEventListener("change", () => {
  invoke("set_autostart", { enabled: els.autostart.checked });
});

els.blockHotkeys.addEventListener("change", () => {
  settings.block_hotkeys = els.blockHotkeys.checked;
  apply();
});

els.showLock.addEventListener("change", () => {
  settings.show_lock = els.showLock.checked;
  apply();
});

els.showPin.addEventListener("change", () => {
  settings.show_pin = els.showPin.checked;
  apply();
});

els.showSettings.addEventListener("change", () => {
  settings.show_settings = els.showSettings.checked;
  apply();
});

els.customCss.addEventListener("input", () => {
  settings.custom_css = els.customCss.value;
  apply();
});

els.resetBtn.addEventListener("click", async () => {
  settings = await invoke("reset_settings");
  invoke("set_always_on_top", { value: settings.always_on_top });
  populateUI();
  emit("settings-changed", settings);
});

els.uiLang.addEventListener("change", () => setUiLang(els.uiLang.value));

els.checkUpdate.addEventListener("click", () => checkForUpdate(true));

els.exitBtn.addEventListener("click", () => invoke("quit_app"));

// Reflect autostart changes made from the tray menu while this panel is open.
listen("autostart-changed", (e) => {
  els.autostart.checked = !!e.payload;
});

// ---------- Update check ----------
const REPO = "moskale-mukola/WindowsLanguageWidget";

// Returns >0 if a>b, <0 if a<b, 0 if equal. Compares dot-separated numbers.
function cmpVersions(a, b) {
  const pa = a.replace(/^v/, "").split(".").map((n) => parseInt(n, 10) || 0);
  const pb = b.replace(/^v/, "").split(".").map((n) => parseInt(n, 10) || 0);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const d = (pa[i] || 0) - (pb[i] || 0);
    if (d) return d;
  }
  return 0;
}

let currentVersion = "";

async function checkForUpdate(manual) {
  const verEl = document.getElementById("version");
  const statusEl = document.getElementById("updateStatus");
  const dict = I18N[getUiLang()] || I18N.en;
  verEl.textContent = "v" + currentVersion;
  if (manual) statusEl.textContent = dict.checking || "Checking…";
  try {
    const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!res.ok) throw new Error("http " + res.status);
    const rel = await res.json();
    const latest = (rel.tag_name || "").replace(/^v/, "");
    if (latest && cmpVersions(latest, currentVersion) > 0) {
      const url = rel.html_url || `https://github.com/${REPO}/releases`;
      const label = (dict.update_available || "Update available:") + " v" + latest;
      statusEl.innerHTML = `<a href="${url}" target="_blank" rel="noopener">${label}</a>`;
    } else {
      statusEl.textContent = manual ? (dict.up_to_date || "Up to date") : "";
    }
  } catch (_) {
    // Offline or rate-limited.
    statusEl.textContent = manual ? (dict.update_failed || "Check failed") : "";
  }
}

// ---------- Init ----------
(async () => {
  const lang = getUiLang();
  els.uiLang.value = lang;
  applyI18n(lang);
  buildSwatches();
  settings = await invoke("get_settings");
  populateUI();
  els.autostart.checked = await invoke("get_autostart");
  try {
    currentVersion = await window.__TAURI__.app.getVersion();
    checkForUpdate(false);
  } catch (_) {}
})();
