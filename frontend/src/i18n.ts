import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import { resources, namespaces } from './locales'

export type SupportedLanguage = 'en' | 'zh'

const LANG_KEY = 'gr-lang'

function detectLanguage(): SupportedLanguage {
  const stored = localStorage.getItem(LANG_KEY) as SupportedLanguage | null
  if (stored === 'en' || stored === 'zh') return stored
  const browser = (navigator.language || 'en').toLowerCase()
  if (browser.startsWith('zh')) return 'zh'
  return 'en'
}

i18n.use(initReactI18next).init({
  resources,
  lng: detectLanguage(),
  fallbackLng: 'en',
  ns: namespaces,
  defaultNS: 'common',
  interpolation: {
    escapeValue: false, // React already escapes
    // Locale files write placeholders as {name}, not i18next's default {{name}}.
    prefix: '{',
    suffix: '}',
  },
})

/** Switch language, persist choice and update <html lang>. */
export function setLanguage(lang: SupportedLanguage) {
  localStorage.setItem(LANG_KEY, lang)
  void i18n.changeLanguage(lang)
  document.documentElement.lang = lang === 'zh' ? 'zh-CN' : 'en'
}

document.documentElement.lang =
  i18n.language === 'zh' ? 'zh-CN' : 'en'

export type Namespace = (typeof namespaces)[number]
export { namespaces }
export default i18n