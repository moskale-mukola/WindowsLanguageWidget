const { invoke } = window.__TAURI__.core;
const { emit } = window.__TAURI__.event;

const PRESETS = ["1E1E1E", "000000", "2D2D30", "0F0F0F", "1A1A2E", "16213E", "0D1117", "3C3C3C"];

const els = {
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

// ---------- Wiring ----------
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

// ---------- Init ----------
(async () => {
  buildSwatches();
  settings = await invoke("get_settings");

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
})();
