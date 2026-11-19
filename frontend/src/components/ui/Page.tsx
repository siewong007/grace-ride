import React, { ReactNode, useContext, useRef } from 'react';
import { Link, NavLink } from 'react-router-dom';
import { ArrowLeft, ChevronRight } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useSmartBack } from '../BackNavigation';
import { GuideHeaderActionContext, usePageChrome } from '../PageChrome';

export interface PageHeaderProps {
  title: ReactNode;
  subtitle?: ReactNode;
  back?: string;
  actions?: ReactNode;
  className?: string;
}

function StandaloneBackLink({ to, label }: { to: string; label: string }) {
  const smartBack = useSmartBack(to);
  return (
    <Link to={smartBack.href} onClick={smartBack.onClick} className="icon-btn sm" aria-label={label}>
      <ArrowLeft size={20} />
    </Link>
  );
}

/**
 * Page heading with optional subtitle and actions. Inside the app frame the back link
 * lives in the top bar, and the top bar shows the title only once this heading scrolls away.
 */
export function PageHeader({ title, subtitle, back, actions, className = '' }: PageHeaderProps) {
  const { t } = useTranslation();
  const headingRef = useRef<HTMLHeadingElement>(null);
  const inFrame = usePageChrome(headingRef, back);
  const guideHeaderAction = useContext(GuideHeaderActionContext);
  return (
    <div className={`page-head ${className}`.trim()}>
      <div className="row nowrap page-head-title">
        {back && !inFrame && (
          <StandaloneBackLink to={back} label={t('back') || 'Back'} />
        )}
        <div className="page-head-text">
          <h1 ref={headingRef}>{title}</h1>
          {subtitle && <p className="muted">{subtitle}</p>}
        </div>
      </div>
      {(actions || guideHeaderAction) && (
        <div className="btn-row page-head-actions">
          {guideHeaderAction}
          {actions}
        </div>
      )}
    </div>
  );
}

export interface PageProps extends React.HTMLAttributes<HTMLDivElement> {
  width?: 'narrow' | 'medium' | 'wide';
  children: ReactNode;
}

/**
 * Standard page wrapper replacing inline max-width layouts.
 */
export function Page({ width = 'wide', className = '', children, ...rest }: PageProps) {
  return (
    <div className={`page-container page-${width} ${className}`.trim()} {...rest}>
      {children}
    </div>
  );
}

/** A list row used by the profile page and the phone admin sheet: icon, label, optional summary line, badge, chevron. */
export function SheetLink({
  to,
  end,
  icon,
  label,
  hint,
  tone,
  badge,
  chevron = true,
  onClick,
  'aria-haspopup': ariaHasPopup,
}: {
  to?: string;
  end?: boolean;
  icon: ReactNode;
  label: ReactNode;
  /** A short summary under the label, such as the current setting. */
  hint?: ReactNode;
  /** Shows the icon on a tinted tile in one of the avatar colours (`purple`, `teal`, …). */
  tone?: string;
  badge?: ReactNode;
  chevron?: boolean;
  onClick?: () => void;
  'aria-haspopup'?: React.AriaAttributes['aria-haspopup'];
}) {
  const body = (
    <>
      <span className={tone ? `sheet-tile av-${tone}` : 'sidebar-icon'}>{icon}</span>
      {hint ? (
        <span className="grow sheet-link-text">
          <span>{label}</span>
          <span className="sheet-link-hint">{hint}</span>
        </span>
      ) : (
        <span className="grow">{label}</span>
      )}
      {badge}
      {chevron ? <ChevronRight size={18} className="muted sheet-link-chevron" aria-hidden="true" /> : null}
    </>
  );
  if (to) {
    return (
      <NavLink
        to={to}
        end={end}
        className={({ isActive }) => `item admin-sheet-link${isActive ? ' active' : ''}`}
        onClick={onClick}
      >
        {body}
      </NavLink>
    );
  }
  return (
    <button type="button" className="item admin-sheet-link" aria-haspopup={ariaHasPopup} onClick={onClick}>
      {body}
    </button>
  );
}
