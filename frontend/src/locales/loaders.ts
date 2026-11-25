export type SupportedLanguage = 'en' | 'zh'

export const CORE_NAMESPACES = [
  'common',
  'layout',
  'auth',
  'home',
  'format',
  'errors',
  'chrome',
  'onboarding',
  'notFound',
  'terms',
  'rideCard',
] as const

export const namespaces = [
  'common',
  'layout',
  'auth',
  'invite',
  'visit',
  'home',
  'rides',
  'rideCard',
  'rideDetail',
  'rideNew',
  'drive',
  'events',
  'eventDetail',
  'eventForm',
  'settlements',
  'costs',
  'reimbursements',
  'help',
  'notifications',
  'profile',
  'report',
  'admin',
  'format',
  'errors',
  'terms',
  'notFound',
  'chrome',
  'plan',
  'onboarding',
  'studio',
] as const

export type Namespace = (typeof namespaces)[number]
export type CoreNamespace = (typeof CORE_NAMESPACES)[number]
export type LazyNamespace = Exclude<Namespace, CoreNamespace>

const CORE_SET = new Set<string>(CORE_NAMESPACES)

export function isCoreNamespace(ns: Namespace): ns is CoreNamespace {
  return CORE_SET.has(ns)
}

export async function loadCoreLocale(
  lang: SupportedLanguage,
): Promise<Record<CoreNamespace, Record<string, unknown>>> {
  const mod = lang === 'zh' ? await import('./core.zh') : await import('./core.en')
  return mod.default as unknown as Record<CoreNamespace, Record<string, unknown>>
}

const lazyLoaders: Record<
  LazyNamespace,
  Record<SupportedLanguage, () => Promise<{ default: Record<string, unknown> }>>
> = {
  invite: { en: () => import('./invite.en'), zh: () => import('./invite.zh') },
  visit: { en: () => import('./visit.en'), zh: () => import('./visit.zh') },
  rides: { en: () => import('./rides.en'), zh: () => import('./rides.zh') },
  rideDetail: { en: () => import('./rideDetail.en'), zh: () => import('./rideDetail.zh') },
  rideNew: { en: () => import('./rideNew.en'), zh: () => import('./rideNew.zh') },
  drive: { en: () => import('./drive.en'), zh: () => import('./drive.zh') },
  events: { en: () => import('./events.en'), zh: () => import('./events.zh') },
  eventDetail: { en: () => import('./eventDetail.en'), zh: () => import('./eventDetail.zh') },
  eventForm: { en: () => import('./eventForm.en'), zh: () => import('./eventForm.zh') },
  settlements: { en: () => import('./settlements.en'), zh: () => import('./settlements.zh') },
  costs: { en: () => import('./costs.en'), zh: () => import('./costs.zh') },
  reimbursements: {
    en: () => import('./reimbursements.en'),
    zh: () => import('./reimbursements.zh'),
  },
  help: { en: () => import('./help.en'), zh: () => import('./help.zh') },
  notifications: {
    en: () => import('./notifications.en'),
    zh: () => import('./notifications.zh'),
  },
  profile: { en: () => import('./profile.en'), zh: () => import('./profile.zh') },
  report: { en: () => import('./report.en'), zh: () => import('./report.zh') },
  admin: { en: () => import('./admin.en'), zh: () => import('./admin.zh') },
  plan: { en: () => import('./plan.en'), zh: () => import('./plan.zh') },
  studio: { en: () => import('./studio.en'), zh: () => import('./studio.zh') },
}

export async function loadLazyNamespace(
  lang: SupportedLanguage,
  ns: LazyNamespace,
): Promise<Record<string, unknown>> {
  const mod = await lazyLoaders[ns][lang]()
  return mod.default
}
