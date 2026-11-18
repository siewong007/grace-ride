import { useTranslation } from 'react-i18next';
import { useApi } from '../../api';

export default function BudgetDashboard() {
  const { t } = useTranslation('admin');
  const { data } = useApi('/budget/summary');
  return (
    <div className="page">
      <h1>{t('budget.title')}</h1>
      <div className="card">
        <p>{t('budget.summary')}</p>
      </div>
    </div>
  );
}
