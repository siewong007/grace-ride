import { useTranslation } from 'react-i18next';
import { useApi } from '../../api';

export default function ExpenseReceipts() {
  const { t } = useTranslation('admin');
  const { data } = useApi('/reimbursements');
  return (
    <div className="page">
      <h1>{t('reimbursements.title')}</h1>
      <p>{t('reimbursements.desc')}</p>
    </div>
  );
}
