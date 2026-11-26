import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ArrowLeft, ArrowRight, ImagePlus, Pencil, Trash2 } from 'lucide-react';
import { del, post, put, uploadFile, useApi } from '../../api';
import { shrinkPhoto } from '../../cropImage';
import { PhotoViewer, type AreaPhoto, type AreaPhotosResponse } from '../../components/AreaPhotos';
import {
  Button,
  Card,
  CardSkeleton,
  EmptyState,
  ErrorBox,
  Field,
  IconButton,
  Modal,
  useAction,
  useConfirm,
} from '../../components/ui';

interface AreaRow {
  id: number;
  name: string;
  active: boolean;
  photo_count?: number;
}

const CAPTION_MAX = 120;

/**
 * Settings → Area photos. The traffic lead, area managers and traffic team members add
 * photos of each pickup area so passengers can recognise where to wait. Team members
 * change or remove only their own; managers reorder and remove any (server decides).
 */
export default function AreaPhotos() {
  const { t } = useTranslation('admin');
  const areas = useApi<AreaRow[]>('/areas');
  const [areaId, setAreaId] = useState<number | null>(null);
  const list = (areas.data ?? []).filter((a) => a.active || (a.photo_count ?? 0) > 0);

  useEffect(() => {
    if (areaId == null && list.length) setAreaId(list[0].id);
  }, [areaId, list]);

  return (
    <Card className="stack">
      <div className="stack-xs">
        <h3>{t('areaPhotos.title')}</h3>
        <p className="muted small">{t('areaPhotos.desc')}</p>
      </div>
      {areas.loading && !areas.data ? (
        <CardSkeleton count={1} />
      ) : !list.length ? (
        <EmptyState title={t('settings.emptyAreas')} />
      ) : (
        <>
          <div className="chips" role="group" aria-label={t('areaPhotos.pickArea')}>
            {list.map((a) => (
              <button
                key={a.id}
                type="button"
                className={`chip ${a.id === areaId ? 'on' : ''}`}
                aria-pressed={a.id === areaId}
                onClick={() => setAreaId(a.id)}
              >
                {a.name}
                {(a.photo_count ?? 0) > 0 && <span className="muted tiny"> · {a.photo_count}</span>}
              </button>
            ))}
          </div>
          {areaId != null && <AreaPhotoManager key={areaId} areaId={areaId} onChanged={areas.reload} />}
        </>
      )}
    </Card>
  );
}

function AreaPhotoManager({ areaId, onChanged }: { areaId: number; onChanged: () => void }) {
  const { t } = useTranslation('admin');
  const confirm = useConfirm();
  const { data, error: loadError, loading, reload } = useApi<AreaPhotosResponse>(`/areas/${areaId}/photos`);
  const { busy, error, setError, run } = useAction();
  const [caption, setCaption] = useState('');
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
  const [editing, setEditing] = useState<AreaPhoto | null>(null);
  const [viewing, setViewing] = useState<number | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);

  if (loading && !data) return <CardSkeleton count={1} />;
  if (!data || !Array.isArray(data.photos)) {
    return <ErrorBox error={loadError ?? error} onRetry={reload} />;
  }

  const photos = data.photos;
  const room = Math.max(0, data.max - photos.length);
  const refresh = async () => {
    await reload();
    onChanged();
  };

  const addFiles = async (files: FileList | null) => {
    const picked = Array.from(files ?? []).filter((f) => f.type.startsWith('image/') || f.type === '');
    if (fileRef.current) fileRef.current.value = '';
    if (!picked.length) return;
    if (picked.length > room) {
      setError(t('areaPhotos.tooMany', { count: room }));
      return;
    }
    setProgress({ done: 0, total: picked.length });
    await run(async () => {
      for (const [i, f] of picked.entries()) {
        const small = await shrinkPhoto(f);
        const up = await uploadFile(small);
        await post(`/areas/${areaId}/photos`, { file_id: up.id, caption: caption.trim() });
        setProgress({ done: i + 1, total: picked.length });
      }
      setCaption('');
    }, t('areaPhotos.added', { count: picked.length }));
    setProgress(null);
    await refresh();
  };

  const move = (from: number, to: number) => {
    const ids = photos.map((p) => p.id);
    const [id] = ids.splice(from, 1);
    ids.splice(to, 0, id);
    run(() => put(`/areas/${areaId}/photos/order`, { photo_ids: ids }).then(refresh));
  };

  const remove = async (p: AreaPhoto) => {
    const ok = await confirm({
      title: t('areaPhotos.removeTitle'),
      body: t('areaPhotos.removeBody'),
      confirmLabel: t('areaPhotos.remove'),
      tone: 'danger',
    });
    if (ok) run(() => del(`/areas/${areaId}/photos/${p.id}`).then(refresh), t('areaPhotos.removed'));
  };

  return (
    <div className="stack">
      <div className="row between wrap gap-xs">
        <strong>{data.area.name}</strong>
        <span className="muted small">{t('areaPhotos.count', { n: photos.length, max: data.max })}</span>
      </div>

      {photos.length === 0 ? (
        <EmptyState title={t('areaPhotos.empty')} description={t('areaPhotos.emptyHint')} />
      ) : (
        <ul className="area-photo-admin-grid">
          {photos.map((p, i) => (
            <li key={p.id} className="area-photo-admin-item">
              <button
                type="button"
                className="area-photo-thumb"
                onClick={() => setViewing(i)}
                aria-label={t('areaPhotos.view', { n: i + 1 })}
              >
                <img src={p.url} alt="" loading="lazy" />
              </button>
              <div className="area-photo-admin-meta">
                <span className={`small ${p.caption ? '' : 'muted'}`}>{p.caption || t('areaPhotos.noCaption')}</span>
                {p.created_by?.name && (
                  <span className="muted tiny">
                    {p.mine ? t('areaPhotos.addedByYou') : t('areaPhotos.addedBy', { name: p.created_by.name })}
                  </span>
                )}
                <div className="area-photo-admin-actions">
                  {data.can_reorder && (
                    <>
                      <IconButton
                        icon={<ArrowLeft size={16} />}
                        size="sm"
                        label={t('areaPhotos.moveEarlier')}
                        disabled={busy || i === 0}
                        onClick={() => move(i, i - 1)}
                      />
                      <IconButton
                        icon={<ArrowRight size={16} />}
                        size="sm"
                        label={t('areaPhotos.moveLater')}
                        disabled={busy || i === photos.length - 1}
                        onClick={() => move(i, i + 1)}
                      />
                    </>
                  )}
                  {p.can_edit && (
                    <IconButton
                      icon={<Pencil size={16} />}
                      size="sm"
                      label={t('areaPhotos.editCaption')}
                      disabled={busy}
                      onClick={() => setEditing(p)}
                    />
                  )}
                  {p.can_delete && (
                    <IconButton
                      icon={<Trash2 size={16} />}
                      size="sm"
                      label={t('areaPhotos.remove')}
                      disabled={busy}
                      onClick={() => remove(p)}
                    />
                  )}
                </div>
              </div>
            </li>
          ))}
        </ul>
      )}

      {data.can_add && (
        room > 0 ? (
          <div className="stack-sm">
            <Field label={t('areaPhotos.caption')} hint={t('areaPhotos.captionHint')}>
              <input
                className="input"
                value={caption}
                maxLength={CAPTION_MAX}
                placeholder={t('areaPhotos.captionPlaceholder')}
                onChange={(e) => setCaption(e.target.value)}
              />
            </Field>
            <input
              ref={fileRef}
              type="file"
              accept="image/jpeg,image/png,image/webp,image/heic,image/*"
              multiple
              className="sr-only"
              tabIndex={-1}
              aria-hidden="true"
              onChange={(e) => addFiles(e.target.files)}
            />
            <Button
              variant="primary"
              icon={<ImagePlus size={16} />}
              disabled={busy}
              loading={!!progress}
              onClick={() => fileRef.current?.click()}
            >
              {progress
                ? t('areaPhotos.uploading', { done: progress.done, total: progress.total })
                : t('areaPhotos.add')}
            </Button>
            <p className="muted tiny">{t('areaPhotos.limits', { max: data.max })}</p>
          </div>
        ) : (
          <p className="muted small">{t('areaPhotos.full', { max: data.max })}</p>
        )
      )}

      <ErrorBox error={error} />

      {editing && (
        <CaptionModal
          photo={editing}
          busy={busy}
          error={error}
          onClose={() => setEditing(null)}
          onSave={(c) =>
            run(
              () =>
                put(`/areas/${areaId}/photos/${editing.id}`, { caption: c }).then(() => {
                  setEditing(null);
                  return refresh();
                }),
              t('settings.saved'),
            )
          }
        />
      )}
      {viewing != null && (
        <PhotoViewer photos={photos} start={viewing} title={data.area.name} onClose={() => setViewing(null)} />
      )}
    </div>
  );
}

function CaptionModal({
  photo,
  busy,
  error,
  onClose,
  onSave,
}: {
  photo: AreaPhoto;
  busy: boolean;
  error: string | null;
  onClose: () => void;
  onSave: (caption: string) => void;
}) {
  const { t } = useTranslation('admin');
  const [caption, setCaption] = useState(photo.caption);
  return (
    <Modal title={t('areaPhotos.editCaption')} onClose={onClose} size="sm">
      <img src={photo.url} alt="" className="area-photo-edit-preview" />
      <Field label={t('areaPhotos.caption')}>
        <input
          className="input"
          value={caption}
          maxLength={CAPTION_MAX}
          placeholder={t('areaPhotos.captionPlaceholder')}
          onChange={(e) => setCaption(e.target.value)}
        />
      </Field>
      <ErrorBox error={error} />
      <Button variant="primary" block disabled={busy} onClick={() => onSave(caption.trim())}>
        {t('settings.save')}
      </Button>
    </Modal>
  );
}
