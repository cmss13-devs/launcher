import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "./locales/en.json";

const resources = {
  en: { translation: en },
} as const;

i18n.use(initReactI18next).init({
  resources,
  lng: navigator.language,
  fallbackLng: "en",
  interpolation: {
    escapeValue: false,
  },
});

export function setLocale(locale: string | null) {
  i18n.changeLanguage(locale ?? navigator.language);
}

export function getAvailableLocales(): string[] {
  return Object.keys(resources);
}

export default i18n;
