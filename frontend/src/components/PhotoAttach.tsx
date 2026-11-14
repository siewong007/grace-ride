import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ImagePlus, X } from 'lucide-react';
import { uploadFile } from '../api';
import { Button } from './ui';

/**
 * Attach one photo to a help message: pick (or take) an image, upload it, show a thumbnail.
 * The parent only sees the uploaded file id; the server checks it is the sender's own image.
 */
export default function PhotoAttach({
  fileId,
  onChange,
  disabled,
}: {
  fileId: number | null;
  onChange: (id: number | null) => void;
  disabled?: boolean;
}) {
  const { t } = useTranslation('help');
  const inputRef = useRef<HTMLInputElement>(null);
  const [uploading, setUploading] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const pick = async (file: File | undefined) => {
    if (!file) return;
    setErr(null);
    setUploading(true);
    try {
      const f = await uploadFile(file);
      onChange(f.id);
    } catch (e) {
      setErr((e as Error).message);
    } finally {
      setUploading(false);
      if (inputRef.current) inputRef.current.value = '';
    }
  };

  return (
    <div className="photo-attach row nowrap gap-sm">
      <input
        ref={inputRef}
        type="file"
        accept="image/*"
        className="sr-only"
        tabIndex={-1}
        aria-hidden="true"
        onChange={(e) => pick(e.target.files?.[0])}
      />
      {fileId ? (
        <span className="photo-attach-chip row nowrap gap-xs">
          <img src={`/api/files/${fileId}`} alt={t('photoAttached')} className="photo-attach-thumb" />
          <span className="small">{t('photoAttached')}</span>
          <Button
            variant="ghost"
            size="sm"
            type="button"
            onClick={() => onChange(null)}
            aria-label={t('photoRemove')}
          >
            <X size={14} />
          </Button>
        </span>
      ) : (
        <Button
          variant="ghost"
          size="sm"
          type="button"
          icon={<ImagePlus size={16} />}
          disabled={disabled || uploading}
          loading={uploading}
          onClick={() => inputRef.current?.click()}
        >
          {t('photoAdd')}
        </Button>
      )}
      {err && <span className="field-error small" role="alert">{err}</span>}
    </div>
  );
}

/** A photo inside a message bubble; opens full size in a new tab. */
export function MessagePhoto({ fileId }: { fileId: number }) {
  const { t } = useTranslation('help');
  return (
    <a href={`/api/files/${fileId}`} target="_blank" rel="noreferrer" className="help-photo-link">
      <img src={`/api/files/${fileId}`} alt={t('photoInMessage')} className="help-photo" loading="lazy" />
    </a>
  );
}
