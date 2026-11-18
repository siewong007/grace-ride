import { useState } from 'react';
import { Copy, Share2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { del, post } from '../../api';
import { fmtDateTime, useFormatT } from '../../format';
import type { Ride } from '../RideCard';
import { Button, ErrorBox, Field, Modal, useAction, useToast } from '../ui';

export function CareLinkModal({ ride, onClose }: { ride: Ride; onClose: () => void }) {
  const { t } = useTranslation('rideDetail');
  const ft = useFormatT();
  const toast = useToast();
  const { busy, error, run } = useAction();
  const [label, setLabel] = useState('');
  const [link, setLink] = useState<{ url: string; valid_until: string } | null>(null);

  const generate = () => {
    run(async () => {
      const res = await post<{ token: string; url: string; valid_until: string }>(
        `/rides/${ride.id}/care-link`,
        { label: label.trim() },
      );
      const origin = typeof window !== 'undefined' ? window.location.origin : '';
      setLink({ url: `${origin}${res.url}`, valid_until: res.valid_until });
    });
  };

  const revoke = () => {
    run(async () => {
      await del(`/rides/${ride.id}/care-link`);
      setLink(null);
    }, t('careLinkRevoked'));
  };

  const copy = () => {
    if (!link || typeof navigator === 'undefined' || !navigator.clipboard) return;
    navigator.clipboard
      .writeText(link.url)
      .then(() => toast.success(t('careLinkCopied')))
      .catch(() => {});
  };

  return (
    <Modal title={t('careLinkTitle')} onClose={onClose}>
      <p className="muted small">{t('careLinkDesc')}</p>
      {!link ? (
        <div className="stack-sm">
          <Field label={t('careLinkLabelField')}>
            <input
              className="input"
              value={label}
              onChange={(e) => setLabel(e.target.value)}
              placeholder={t('careLinkLabelPlaceholder')}
            />
          </Field>
          <ErrorBox error={error} />
          <div className="btn-row">
            <Button variant="primary" disabled={busy} onClick={generate}>
              {t('careLinkGenerate')}
            </Button>
            <Button variant="ghost" disabled={busy} onClick={revoke}>
              {t('careLinkRevokeOld')}
            </Button>
          </div>
        </div>
      ) : (
        <div className="stack-sm">
          <input className="input mono" readOnly value={link.url} />
          {link.valid_until && (
            <p className="muted tiny no-margin">{fmtDateTime(link.valid_until, ft)}</p>
          )}
          <ErrorBox error={error} />
          <div className="btn-row">
            <Button variant="primary" icon={<Copy size={16} />} onClick={copy}>
              {t('careLinkCopy')}
            </Button>
            <Button
              variant="secondary"
              icon={<Share2 size={16} />}
              onClick={() =>
                window.open(
                  `https://wa.me/?text=${encodeURIComponent(`${t('careLinkShareText')}: ${link.url}`)}`,
                  '_blank',
                  'noopener,noreferrer',
                )
              }
            >
              {t('shareWhatsApp')}
            </Button>
            <Button variant="danger" size="sm" disabled={busy} onClick={revoke}>
              {t('careLinkRevokeOld')}
            </Button>
          </div>
        </div>
      )}
    </Modal>
  );
}
