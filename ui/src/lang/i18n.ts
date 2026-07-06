import i18n from "i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import { initReactI18next } from "react-i18next";
import en from "./en.json";
import vi from "./vi.json";

export const initI18n = (initialLng = "") => {
  return i18n
    .use(LanguageDetector)
    .use(initReactI18next)
    .init({
      resources: {
        vi: { translation: vi },
        en: { translation: en },
      },
      fallbackLng: "vi",
      supportedLngs: ["vi", "en"],
      ...(initialLng ? { lng: initialLng } : {}),
      detection: {
        order: ["localStorage", "navigator"],
        caches: ["localStorage"],
        lookupLocalStorage: "gd-language",
      },
      interpolation: {
        escapeValue: false,
      },
      returnNull: false,
      returnEmptyString: false,
      saveMissing: import.meta.env.DEV,
      missingKeyHandler: (lng, ns, key) => {
        if (import.meta.env.DEV) {
          console.warn("[i18n] missing key", { lng, ns, key });
        }
      },
    });
};

export default i18n;
