import { useSearchParams } from 'react-router-dom';
import { useApi } from '../../api';
import { Empty, ErrorBox, Spinner } from '../../components/ui';
import { fmtDateTime, useFormatT } from '../../format';
import { useTranslation } from 'react-i18next';

export default function Audit() {
  const { t } = useTranslation('admin');
  const ft = useFormatT();
  const [params] = useSearchParams();
  const q = params.toString();
  const { data, error, loading } = useApi<{ id: number; actor: string | null; action: string; entity: string; entity_id: number | null; detail: string; created_at: string }[]>(`/admin/audit${q ? `?${q}` : ''}`);
  return (
    <div className="stack">
      <div className="page-head"><div><h1>{t('audit.title')}</h1><p>{t('audit.desc')}</p></div></div>
      <ErrorBox error={error} />
      <div className="card">
        {loading ? <Spinner /> : !data?.length ? <Empty>{t('audit.none')}</Empty> : (
          <div className="table-wrap">
            <table className="table responsive">
              <thead><tr><th>{t('audit.time')}</th><th>{t('audit.actor')}</th><th>{t('audit.action')}</th><th>{t('audit.object')}</th><th>{t('audit.detail')}</th></tr></thead>
              <tbody>
                {data.map((a) => (
                  <tr key={a.id}>
                    <td data-label={t('audit.time')} className="small">{fmtDateTime(a.created_at, ft)}</td>
                    <td data-label={t('audit.actor')}>{a.actor ?? t('audit.system')}</td>
                    <td data-label={t('audit.action')} className="mono small">{a.action}</td>
                    <td data-label={t('audit.object')} className="small">{a.entity}{a.entity_id != null && ` #${a.entity_id}`}</td>
                    <td data-label={t('audit.detail')} className="small">{a.detail}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
