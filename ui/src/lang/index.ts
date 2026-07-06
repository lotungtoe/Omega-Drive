import i18n from "./i18n";

export function setLanguage(lang = "") {
  if (!lang) return;
  i18n.changeLanguage(lang);
  try {
    localStorage.setItem("gd-language", lang);
  } catch {
    // ignore storage errors
  }
}

export const LANG_OPTIONS = [
  { value: "vi", label: "Vietnamese" },
  { value: "en", label: "English" },
];

export { i18n };
export { initI18n } from "./i18n";
