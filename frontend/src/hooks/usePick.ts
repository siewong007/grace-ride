import { useTranslation } from 'react-i18next';

/** Pick the active language, falling back to the other when empty. */
export function usePick() {
  const { i18n } = useTranslation();
  return (zh: string, en: string) => (i18n.language.startsWith('zh') ? zh || en : en || zh);
}
