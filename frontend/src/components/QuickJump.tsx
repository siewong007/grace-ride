import { KeyboardEvent, ReactNode, useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import {
  BookOpen,
  Calendar,
  CheckCircle2,
  CreditCard,
  DollarSign,
  LifeBuoy,
  Receipt,
  Search,
  Shield,
  UserPlus,
  Users,
} from 'lucide-react';
import { useApi } from '../api';
import { useAuth } from '../auth';
import type { GuideSummary } from '../types/help';
import { usePick } from '../hooks/usePick';
import { Modal } from './ui';

interface JumpPage {
  to: string;
  label: string;
  icon: ReactNode;
}

interface Result {
  key: string;
  to: string;
  label: string;
  group: string;
  icon: ReactNode;
}

/** Opens on Ctrl/Cmd+K (or the topbar search button) and jumps to any page, guide or help request. */
export function useQuickJumpShortcut(open: () => void) {
  useEffect(() => {
    const onKey = (e: globalThis.KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        try {
          if (localStorage.getItem('gr_locked_user')) return;
        } catch {
          /* ignore */
        }
        e.preventDefault();
        open();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open]);
}

export default function QuickJump({ pages, onClose }: { pages: JumpPage[]; onClose: () => void }) {
  const { t } = useTranslation('layout');
  const nav = useNavigate();
  const pick = usePick();
  const { can } = useAuth();
  const [q, setQ] = useState('');
  const [active, setActive] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const guidesData = useApi<GuideSummary[]>(can('help') ? '/help/guides' : null).data;
  const helpDesk = can('admin_help');
  const requestsData = useApi<{ id: number; subject: string; user_name: string }[]>(
    can('help') || helpDesk ? `/help${helpDesk ? '?all=true' : ''}` : null,
  ).data;

  const quickActions = useMemo<Result[]>(() => {
    const list: { key: string; to: string; label: string; keywords: string; icon: ReactNode; canView: boolean }[] = [
      {
        key: 'act-pending-rides',
        to: '/admin/rides?status=pending_approval',
        label: t('actionPendingRides'),
        keywords: 'rides pending approve',
        icon: <CheckCircle2 size={18} />,
        canView: can('admin_rides'),
      },
      {
        key: 'act-sunday-rides',
        to: '/admin/rides?date=upcoming_sunday',
        label: t('actionSundayRides'),
        keywords: 'sunday rides transport service schedule',
        icon: <Calendar size={18} />,
        canView: can('admin_rides'),
      },
      {
        key: 'act-pending-members',
        to: '/admin/members?status=pending',
        label: t('actionPendingMembers'),
        keywords: 'members pending approval users verify',
        icon: <Users size={18} />,
        canView: can('admin_members'),
      },
      {
        key: 'act-invite-member',
        to: '/admin/members?tab=invites',
        label: t('actionInviteMember'),
        keywords: 'invite whatsapp member link new',
        icon: <UserPlus size={18} />,
        canView: can('admin_invites'),
      },
      {
        key: 'act-review-reimb',
        to: '/admin/reimbursements?tab=review',
        label: t('actionReviewReimb'),
        keywords: 'reimbursement review claim receipt expense',
        icon: <Receipt size={18} />,
        canView: can('admin_reimb_review'),
      },
      {
        key: 'act-payout-batches',
        to: '/admin/reimbursements?tab=payout',
        label: t('actionPayoutBatches'),
        keywords: 'payout batches bank transfer duitnow pay payment',
        icon: <CreditCard size={18} />,
        canView: can('admin_reimb_pay'),
      },
      {
        key: 'act-area-fares',
        to: '/admin/settings?tab=fares',
        label: t('actionAreaFares'),
        keywords: 'fares area matrix pricing mileage rate',
        icon: <DollarSign size={18} />,
        canView: can('admin_fares'),
      },
      {
        key: 'act-audit-logs',
        to: '/admin/audit',
        label: t('actionAuditLogs'),
        keywords: 'audit security logs activity history',
        icon: <Shield size={18} />,
        canView: can('admin_audit'),
      },
    ];

    return list
      .filter((a) => a.canView)
      .map((a) => ({
        key: a.key,
        to: a.to,
        label: a.label,
        group: t('jumpActions'),
        icon: a.icon,
        keywords: a.keywords,
      }));
  }, [can, t]);

  const results = useMemo<Result[]>(() => {
    const rawNeedle = q.trim();
    const isCommandMode = rawNeedle.startsWith('>');
    const needle = (isCommandMode ? rawNeedle.slice(1) : rawNeedle).trim().toLowerCase();
    const hit = (s: string) => !needle || s.toLowerCase().includes(needle);

    const matchedActions = quickActions.filter(
      (a) => hit(`${a.label} ${(a as { keywords?: string }).keywords ?? ''}`),
    );

    if (isCommandMode) {
      return matchedActions;
    }

    const guides = guidesData ?? [];
    const requests = requestsData ?? [];
    const page = pages
      .filter((p) => hit(p.label))
      .map((p) => ({ key: `p${p.to}`, to: p.to, label: p.label, group: t('jumpPages'), icon: p.icon }));
    const guide = guides
      .filter((g) => hit(`${g.title_en} ${g.title_zh}`))
      .slice(0, 6)
      .map((g) => ({
        key: `g${g.id}`, to: `/help/guides/${g.slug}`, label: pick(g.title_zh, g.title_en),
        group: t('jumpGuides'), icon: <BookOpen size={18} />,
      }));
    const request = needle
      ? requests
          .filter((r) => hit(`${r.subject} ${r.user_name}`))
          .slice(0, 6)
          .map((r) => ({
            key: `r${r.id}`, to: `/help/${r.id}`, label: helpDesk ? `${r.subject} · ${r.user_name}` : r.subject,
            group: t('jumpRequests'), icon: <LifeBuoy size={18} />,
          }))
      : [];

    return [...matchedActions, ...page, ...guide, ...request];
  }, [q, quickActions, pages, guidesData, requestsData, helpDesk, pick, t]);

  const go = (r: Result) => {
    onClose();
    nav(r.to);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setActive((i) => Math.min(i + 1, results.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setActive((i) => Math.max(i - 1, 0));
    } else if (e.key === 'Enter' && results[active]) {
      e.preventDefault();
      go(results[active]);
    }
  };

  const activeId = results[active] ? `jump-${results[active].key}` : undefined;
  let lastGroup = '';

  return (
    <Modal title={t('jumpTitle')} onClose={onClose} size="md" initialFocusRef={inputRef}>
      <div className="stack">
        <div className="help-search-box">
          <Search size={18} className="help-search-icon" aria-hidden="true" />
          <input
            ref={inputRef}
            className="input help-search-input"
            type="search"
            role="combobox"
            aria-expanded={results.length > 0}
            aria-controls={results.length > 0 ? 'jump-results' : undefined}
            aria-activedescendant={activeId}
            aria-label={t('jumpPlaceholder')}
            placeholder={t('jumpPlaceholder')}
            value={q}
            onChange={(e) => {
              setQ(e.target.value);
              setActive(0);
            }}
            onKeyDown={onKeyDown}
          />
        </div>
        <p className="sr-only" role="status" aria-live="polite">
          {t('jumpCount', { count: results.length })}
        </p>
        {results.length === 0 ? (
          <p className="muted">{t('jumpNone')}</p>
        ) : (
          <ul id="jump-results" role="listbox" className="list jump-list" aria-label={t('jumpTitle')}>
            {results.map((r, i) => {
              const header = r.group !== lastGroup ? r.group : null;
              lastGroup = r.group;
              return (
                <li key={r.key} role="presentation">
                  {header && <div className="section-label jump-group">{header}</div>}
                  <div
                    id={`jump-${r.key}`}
                    role="option"
                    aria-selected={i === active}
                    className={`item row nowrap jump-item ${i === active ? 'active' : ''}`}
                    tabIndex={-1}
                    onMouseEnter={() => setActive(i)}
                    onClick={() => go(r)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') go(r);
                    }}
                  >
                    <span className="sidebar-icon" aria-hidden="true">{r.icon}</span>
                    <span className="grow ellipsis">{r.label}</span>
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </Modal>
  );
}
