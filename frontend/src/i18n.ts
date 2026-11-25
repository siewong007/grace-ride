import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import {
  CORE_NAMESPACES,
  isCoreNamespace,
  loadCoreLocale,
  loadLazyNamespace,
  namespaces,
  type Namespace,
  type SupportedLanguage,
} from './locales/loaders'

export type { Namespace, SupportedLanguage }
export { namespaces }

const LANG_KEY = 'gr-lang'

function normalizeLanguage(raw?: string): SupportedLanguage {
  return raw?.toLowerCase().startsWith('zh') ? 'zh' : 'en'
}

function detectLanguage(): SupportedLanguage {
  const stored = localStorage.getItem(LANG_KEY) as SupportedLanguage | null
  if (stored === 'en' || stored === 'zh') return stored
  const browser = (navigator.language || 'en').toLowerCase()
  if (browser.startsWith('zh')) return 'zh'
  return 'en'
}

const initialLang = detectLanguage()

void i18n.use(initReactI18next).init({
  resources: {},
  lng: initialLang,
  fallbackLng: 'en',
  ns: [...CORE_NAMESPACES],
  defaultNS: 'common',
  react: {
    useSuspense: false,
  },
  interpolation: {
    escapeValue: false, // React already escapes
    // Locale files write placeholders as {name}, not i18next's default {{name}}.
    prefix: '{',
    suffix: '}',
  },
})

const loadedNamespaces = new Set<Namespace>(CORE_NAMESPACES)
const inflight = new Map<string, Promise<void>>()

async function loadCoreInto(lang: SupportedLanguage): Promise<void> {
  const coreReady = CORE_NAMESPACES.every((ns) => i18n.hasResourceBundle(lang, ns))
  if (coreReady) return
  const key = `core:${lang}`
  let task = inflight.get(key)
  if (!task) {
    task = loadCoreLocale(lang)
      .then((bundles) => {
        for (const ns of CORE_NAMESPACES) {
          i18n.addResourceBundle(lang, ns, bundles[ns], true, true)
        }
      })
      .finally(() => {
        inflight.delete(key)
      })
    inflight.set(key, task)
  }
  await task
}

export async function ensureLocaleNamespaces(
  nsList: readonly Namespace[],
  lang?: SupportedLanguage,
): Promise<void> {
  const targetLang = lang ?? normalizeLanguage(i18n.language)
  const langs: SupportedLanguage[] = [targetLang]

  await Promise.all(langs.map((l) => loadCoreInto(l)))

  const tasks: Promise<void>[] = []
  for (const ns of nsList) {
    loadedNamespaces.add(ns)
    if (isCoreNamespace(ns)) continue
    for (const l of langs) {
      if (i18n.hasResourceBundle(l, ns)) continue
      const key = `${l}:${ns}`
      let task = inflight.get(key)
      if (!task) {
        task = loadLazyNamespace(l, ns)
          .then((bundle) => {
            i18n.addResourceBundle(l, ns, bundle, true, true)
          })
          .finally(() => {
            inflight.delete(key)
          })
        inflight.set(key, task)
      }
      tasks.push(task)
    }
  }
  if (tasks.length > 0) {
    await Promise.all(tasks)
  }
}

let initPromise: Promise<void> | null = null

export function initI18n(): Promise<void> {
  if (!initPromise) {
    initPromise = ensureLocaleNamespaces(CORE_NAMESPACES, initialLang)
  }
  return initPromise
}

/** Switch language, persist choice and update <html lang>. */
export async function setLanguage(lang: SupportedLanguage): Promise<void> {
  localStorage.setItem(LANG_KEY, lang)
  document.documentElement.lang = lang === 'zh' ? 'zh-CN' : 'en'
  await ensureLocaleNamespaces([...loadedNamespaces], lang)
  await i18n.changeLanguage(lang)
}

document.documentElement.lang = initialLang === 'zh' ? 'zh-CN' : 'en'

export default i18n
