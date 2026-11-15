import { useTranslation } from 'react-i18next';
import { SheetLink } from '../../components/ui';
import { useStaffNav, waitingBadge } from '../../staffNav';

/** Phone admin menu: grouped staff pages, matching the profile list. */
export default function AdminMenu() {
  const { t } = useTranslation('layout');
  const { staffGroups } = useStaffNav();

  return (
    <div className="stack profile-page">
      <h1 className="sr-only">{t('adminSheetTitle')}</h1>
      {staffGroups.map((g) => (
        <section key={g.key} className="stack-xs" aria-label={g.label}>
          <div className="section-label">{g.label}</div>
          <div className="list">
            {g.items.map((n) => (
              <SheetLink
                key={n.to}
                to={n.to}
                end={n.end}
                icon={n.icon}
                label={n.label}
                badge={waitingBadge(n.badge, t('waitingCount', { count: n.badge ?? 0 }))}
              />
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}
