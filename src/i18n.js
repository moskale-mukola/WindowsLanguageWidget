const I18N = {
  en: {
    ui_lang: "Interface language",
    h_widget: "Widget settings",
    theme: "Theme",
    theme_default: "Default",
    theme_win11_light: "Windows 11 Light",
    theme_win11_dark: "Windows 11 Dark",
    theme_minimal: "Minimal",
    theme_futuristic: "Futuristic",
    size: "Size",
    opacity: "Opacity",
    radius: "Corner radius",
    bg_color: "Background color",
    aot: "Always on top",
    autostart: "Start with Windows",
    block_hotkeys: "Block layout hotkeys while locked",
    registry_block: "Disable system hotkey while locked",
    registry_warn: `⚠ Experimental. Temporarily edits the Windows registry (<code>HKCU\\Keyboard Layout\\Toggle</code>) while
    locked — the most reliable block for games. Your original values are restored on unlock, exit, and next
    launch/logon after a crash. If Alt+Shift ever stays off: Settings → Time &amp; language → Typing →
    Advanced keyboard settings → Input language hot keys.`,
    h_buttons: "Visible buttons",
    btn_lock: "Lock button",
    btn_pin: "Pin button",
    btn_settings: "Settings button",
    h_css: "Custom CSS",
    css_label: "Advanced styling",
    css_placeholder: "/* e.g. #card { font-family: monospace; } */",
    reset: "Reset to defaults",
    hint: "Changes apply instantly. Right-click the widget to open settings.",
  },
  uk: {
    ui_lang: "Мова інтерфейсу",
    h_widget: "Налаштування віджета",
    theme: "Тема",
    theme_default: "Стандартна",
    theme_win11_light: "Windows 11 Світла",
    theme_win11_dark: "Windows 11 Темна",
    theme_minimal: "Мінімалістична",
    theme_futuristic: "Футуристична",
    size: "Розмір",
    opacity: "Прозорість",
    radius: "Заокруглення кутів",
    bg_color: "Колір фону",
    aot: "Поверх усіх вікон",
    autostart: "Запускати з Windows",
    block_hotkeys: "Блокувати гарячі клавіші під час блокування",
    registry_block: "Вимикати системну гарячу клавішу під час блокування",
    registry_warn: `⚠ Експериментально. Тимчасово змінює реєстр Windows (<code>HKCU\\Keyboard Layout\\Toggle</code>)
    під час блокування — найнадійніший спосіб для ігор. Оригінальні значення відновлюються при розблокуванні,
    закритті та наступному запуску після збою. Якщо Alt+Shift перестане перемикати мову: Параметри → Час і мова →
    Введення тексту → Додаткові параметри клавіатури → Комбінації клавіш для мов введення.`,
    h_buttons: "Видимі кнопки",
    btn_lock: "Кнопка блокування",
    btn_pin: "Кнопка закріплення",
    btn_settings: "Кнопка налаштувань",
    h_css: "Власний CSS",
    css_label: "Розширене стилізування",
    css_placeholder: "/* напр. #card { font-family: monospace; } */",
    reset: "Скинути до типових",
    hint: "Зміни застосовуються миттєво. ПКМ по віджету відкриває налаштування.",
  },
};

function applyI18n(lang) {
  const dict = I18N[lang] || I18N.en;
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    const key = el.dataset.i18n;
    if (dict[key] != null) el.innerHTML = dict[key];
  });
  document.querySelectorAll("[data-i18n-placeholder]").forEach((el) => {
    const key = el.dataset.i18nPlaceholder;
    if (dict[key] != null) el.setAttribute("placeholder", dict[key]);
  });
  document.documentElement.lang = lang;
}

function getUiLang() {
  return localStorage.getItem("ui_lang") || "en";
}

function setUiLang(lang) {
  localStorage.setItem("ui_lang", lang);
  applyI18n(lang);
}
