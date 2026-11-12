import { useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useApi } from '../../api';
import {
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Page,
  PageHeader,
} from '../../components/ui';
import { fmtDateTime, useFormatT } from '../../format';

interface AuditItem {
  id: number;
  actor: string | null;
  action: string;
  entity: string;
  entity_id: number | null;
  detail: string;
  created_at: string;
}

export default function Audit() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const [params] = useSearchParams();
  const [search, setSearch] = useState('');

  const q = params.toString();
  const { data, error, loading } = useApi<AuditItem[]>(
    `/admin/audit${q ? `?${q}` : ''}`,
  );

  const filtered = useMemo(() => {
    if (!data) return [];
    if (!search.trim()) return data;
    const term = search.toLowerCase();
    return data.filter(
      (a) =>
        (a.actor && a.actor.toLowerCase().includes(term)) ||
        a.action.toLowerCase().includes(term) ||
        a.entity.toLowerCase().includes(term) ||
        (a.detail && a.detail.toLowerCase().includes(term)),
    );
  }, [data, search]);

  return (
    <Page>
      <PageHeader title={t('audit.title')} subtitle={t('audit.desc')} />

      {/* Search box */}
      <Card className="tight">
        <input
          className="input"
          placeholder={t('audit.search')}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          aria-label={t('audit.search')}
        />
      </Card>

      <ErrorBox error={error} />

      <Card>
        {loading ? (
          <CardSkeleton count={4} />
        ) : !filtered.length ? (
          <EmptyState title={t('audit.none')} />
        ) : (
          <div className="table-wrap">
            <table className="table responsive sticky-header">
              <thead>
                <tr>
                  <th>{t('audit.time')}</th>
                  <th>{t('audit.actor')}</th>
                  <th>{t('audit.action')}</th>
                  <th>{t('audit.object')}</th>
                  <th>{t('audit.detail')}</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map((a) => (
                  <tr key={a.id}>
                    <td data-label={t('audit.time')} className="mono small">
                      {fmtDateTime(a.created_at, ft)}
                    </td>
                    <td data-label={t('audit.actor')}>
                      {a.actor ?? t('audit.system')}
                    </td>
                    <td data-label={t('audit.action')} className="mono small">
                      {a.action}
                    </td>
                    <td data-label={t('audit.object')} className="small">
                      {a.entity}
                      {a.entity_id != null && ` #${a.entity_id}`}
                    </td>
                    <td data-label={t('audit.detail')} className="small">
                      {a.detail}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Card>
    </Page>
  );
}
