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
    block_hotkeys: "Hard-block layout switch while locked",
    block_hotkeys_warn: `⚠ Experimental. While locked, this fully disables the Windows language hotkey — via a
    keyboard hook and by temporarily editing the registry (<code>HKCU\\Keyboard Layout\\Toggle</code> +
    <code>SPI_SETLANGTOGGLE</code>) — so <code>Alt+Shift</code> / <code>Ctrl+Shift</code> never fire a switch and
    games never see a language change. <code>Win+Space</code> and the taskbar indicator keep working. Your
    original settings are restored on unlock, exit, and the next launch/logon after a crash. If the hotkey ever
    stays off: Settings → Time &amp; language → Typing → Advanced keyboard settings → Input language hot keys.`,
    update_available: "Update available:",
    h_buttons: "Visible buttons",
    btn_lock: "Lock button",
    btn_pin: "Pin button",
    btn_settings: "Settings button",
    h_css: "Custom CSS",
    css_label: "Advanced styling",
    css_placeholder: "/* e.g. #card { font-family: monospace; } */",
    reset: "Reset to defaults",
    check_update: "Check for updates",
    exit: "Exit app",
    checking: "Checking…",
    up_to_date: "Up to date",
    update_failed: "Check failed",
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
    block_hotkeys: "Жорстке блокування зміни мови під час блокування",
    block_hotkeys_warn: `⚠ Експериментально. Під час блокування повністю вимикає системну гарячу клавішу мови —
    через клавіатурний хук і тимчасову зміну реєстру (<code>HKCU\\Keyboard Layout\\Toggle</code> +
    <code>SPI_SETLANGTOGGLE</code>) — тому <code>Alt+Shift</code> / <code>Ctrl+Shift</code> взагалі не перемикають
    мову, і гра не бачить зміни. <code>Win+Space</code> і індикатор на панелі задач працюють. Оригінальні
    значення відновлюються при розблокуванні, закритті та наступному запуску після збою. Якщо клавіша лишиться
    вимкненою: Параметри → Час і мова → Введення тексту → Додаткові параметри клавіатури → Комбінації клавіш.`,
    update_available: "Доступне оновлення:",
    h_buttons: "Видимі кнопки",
    btn_lock: "Кнопка блокування",
    btn_pin: "Кнопка закріплення",
    btn_settings: "Кнопка налаштувань",
    h_css: "Власний CSS",
    css_label: "Розширене стилізування",
    css_placeholder: "/* напр. #card { font-family: monospace; } */",
    reset: "Скинути до типових",
    check_update: "Перевірити оновлення",
    exit: "Вийти з додатку",
    checking: "Перевірка…",
    up_to_date: "Найновіша версія",
    update_failed: "Не вдалося перевірити",
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
