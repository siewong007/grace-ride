import { ReactNode, useId, useState } from 'react';
import { Paperclip } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { uploadFile } from '../../api';

export interface FieldProps {
  label: string;
  hint?: ReactNode;
  error?: ReactNode;
  as?: 'label' | 'div' | 'fieldset';
  children: ReactNode;
}

export function Field({
  label,
  hint,
  error,
  as = 'label',
  children,
}: FieldProps) {
  const labelId = useId();
  const className = `field ${error ? 'has-error' : ''}`.trim();
  const feedback = (
    <>
      {hint && !error && <small className="hint">{hint}</small>}
      {error && <span className="field-error" role="alert">{error}</span>}
    </>
  );

  if (as === 'fieldset') {
    return (
      <fieldset className={className}>
        <legend>{label}</legend>
        {children}
        {feedback}
      </fieldset>
    );
  }

  if (as === 'div') {
    return (
      <div className={className} role="group" aria-labelledby={labelId}>
        <span id={labelId}>{label}</span>
        {children}
        {feedback}
      </div>
    );
  }

  return (
    <label className={className}>
      <span>{label}</span>
      {children}
      {feedback}
    </label>
  );
}

export function FileInput({
  label,
  value,
  onChange,
  accept = 'image/*,application/pdf',
}: {
  label: string;
  value: number | null;
  onChange: (id: number | null) => void;
  accept?: string;
}) {
  const { t } = useTranslation();
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [name, setName] = useState<string>('');
  return (
    <div className="field">
      <span>{label}</span>
      <label className="check upload">
        <input
          type="file"
          accept={accept}
          className="sr-only"
          onChange={async (e) => {
            const f = e.target.files?.[0];
            if (!f) return;
            setBusy(true);
            setErr(null);
            try {
              const r = await uploadFile(f);
              setName(f.name);
              onChange(r.id);
            } catch (x) {
              setErr((x as Error).message);
            } finally {
              setBusy(false);
            }
          }}
        />
        <Paperclip size={16} aria-hidden="true" />
        {busy ? t('uploading') : value ? t('uploaded', { name: name || '' }).trim() : t('chooseFile')}
      </label>
      {err && <small className="error">{err}</small>}
    </div>
  );
}

export function ReceiptLink({ id, label }: { id?: number | null; label?: string }) {
  const { t } = useTranslation();
  if (!id) return null;
  return (
    <a className="btn sm" href={`/api/files/${id}`} target="_blank" rel="noreferrer">
      {label ?? t('viewReceipt')}
    </a>
  );
}
