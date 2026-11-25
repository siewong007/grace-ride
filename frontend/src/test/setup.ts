import '@testing-library/jest-dom/vitest';
import i18n from '../i18n';
import { resources } from '../locales';

for (const [lang, nsMap] of Object.entries(resources)) {
  for (const [ns, bundle] of Object.entries(nsMap)) {
    i18n.addResourceBundle(lang, ns, bundle, true, true);
  }
}
