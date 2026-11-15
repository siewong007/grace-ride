import { useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { usePageChrome } from '../../components/PageChrome';
import { SheetLink } from '../../components/ui';
import { useStaffNav, waitingBadge } from '../../staffNav';

/** Phone admin menu: the same grouped list as the profile page, as a full page rather than a sheet. */
export default function AdminMenu() {
  const { t } = useTranslation('layout');
  const headingRef = useRef<HTMLHeadingElement>(null);
  usePageChrome(headingRef);
  const { staffGroups, staffWaiting } = useStaffNav();

  return (
    <div className="stack profile-page">
      <div className="profile-identity">
        <div className="stack-sm">
          <h1 ref={headingRef}>{t('adminSheetTitle')}</h1>
          {staffWaiting > 0 && (
            <p className="muted">{t('waitingCount', { count: staffWaiting })}</p>
          )}
        </div>
      </div>

      <div className="stack">
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
    </div>
  );
}
