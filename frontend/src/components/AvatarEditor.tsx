import { KeyboardEvent as ReactKeyboardEvent, PointerEvent as ReactPointerEvent, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Check, ImagePlus, Trash2, ZoomIn, ZoomOut } from 'lucide-react';
import { put, uploadFile } from '../api';
import { ANIMATED_AVATARS, AVATAR_COLORS, AVATAR_ICONS, type AvatarFields, isAnimatedAvatar } from '../avatarStyle';
import { clampOffset, coverSize, cropToFile, loadImage } from '../cropImage';
import { Avatar, Button, ErrorBox, Modal, Tabs, useAction } from './ui';

type Mode = 'photo' | 'animated' | 'initials' | 'icon';

/** Side of the crop circle, in CSS pixels. */
const VIEW = 240;

/** Pick how your avatar looks: a cropped photo, your initial on a colour, or an icon on a colour. */
export default function AvatarEditor({
  name,
  current,
  onClose,
  onSaved,
}: {
  name: string;
  current: AvatarFields;
  onClose: () => void;
  onSaved: () => void | Promise<unknown>;
}) {
  const { t } = useTranslation('profile');
  const { busy, error, run } = useAction();
  const [mode, setMode] = useState<Mode>(
    current.photo_file_id
      ? 'photo'
      : isAnimatedAvatar(current.avatar_icon)
      ? 'animated'
      : current.avatar_icon
      ? 'icon'
      : 'initials',
  );
  const [color, setColor] = useState<string | null>(current.avatar_color ?? null);
  const [icon, setIcon] = useState<string | null>(current.avatar_icon ?? null);
  const [picked, setPicked] = useState<{ img: HTMLImageElement; url: string; file: File } | null>(null);
  const [crop, setCrop] = useState({ zoom: 1, x: 0, y: 0 });
  const [localErr, setLocalErr] = useState<string | null>(null);
  const drag = useRef<{ px: number; py: number; x: number; y: number } | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);

  useEffect(() => () => {
    if (picked) URL.revokeObjectURL(picked.url);
  }, [picked]);

  const setOffset = (zoom: number, x: number, y: number) => {
    if (!picked) return;
    const c = clampOffset(picked.img.naturalWidth, picked.img.naturalHeight, { view: VIEW, zoom, x, y });
    setCrop({ zoom, ...c });
  };

  const pick = async (file?: File) => {
    if (!file) return;
    setLocalErr(null);
    try {
      setPicked(await loadImage(file));
      setCrop({ zoom: 1, x: 0, y: 0 });
    } catch {
      setLocalErr(t('avatarUnreadable'));
    }
  };

  const onPointerDown = (e: ReactPointerEvent<HTMLButtonElement>) => {
    if (!picked) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = { px: e.clientX, py: e.clientY, x: crop.x, y: crop.y };
  };
  const onPointerMove = (e: ReactPointerEvent<HTMLButtonElement>) => {
    const d = drag.current;
    if (!d) return;
    setOffset(crop.zoom, d.x + e.clientX - d.px, d.y + e.clientY - d.py);
  };
  const onPointerUp = () => {
    drag.current = null;
  };
  const onKeyDown = (e: ReactKeyboardEvent<HTMLButtonElement>) => {
    const step = e.shiftKey ? 20 : 5;
    const moves: Record<string, [number, number]> = {
      ArrowLeft: [-step, 0],
      ArrowRight: [step, 0],
      ArrowUp: [0, -step],
      ArrowDown: [0, step],
    };
    const m = moves[e.key];
    if (!m || !picked) return;
    e.preventDefault();
    setOffset(crop.zoom, crop.x + m[0], crop.y + m[1]);
  };

  const colorField = color ?? '';
  const save = () =>
    run(async () => {
      setLocalErr(null);
      if (mode === 'photo') {
        if (picked) {
          const file = picked.file?.type === 'image/gif'
            ? picked.file
            : await cropToFile(picked.img, { view: VIEW, ...crop });
          const up = await uploadFile(file);
          await put('/me', { photo_file_id: up.id, avatar_color: colorField });
        } else if (current.photo_file_id) {
          await put('/me', { avatar_color: colorField });
        } else {
          throw new Error(t('avatarPickPhoto'));
        }
      } else if (mode === 'icon' || mode === 'animated') {
        if (!icon) throw new Error(t('avatarPickIcon'));
        await put('/me', { clear_photo: true, avatar_icon: icon, avatar_color: colorField });
      } else {
        await put('/me', { clear_photo: true, avatar_icon: '', avatar_color: colorField });
      }
      await onSaved();
      onClose();
    }, t('avatarSaved'));

  const remove = () =>
    run(async () => {
      await put('/me', { clear_photo: true, avatar_icon: '', avatar_color: '' });
      await onSaved();
      onClose();
    }, t('avatarRemoved'));

  const hasCustom = !!(current.photo_file_id || current.avatar_icon || current.avatar_color);
  const size = picked ? coverSize(picked.img.naturalWidth, picked.img.naturalHeight, VIEW, crop.zoom) : null;

  return (
    <Modal
      title={t('avatarTitle')}
      onClose={onClose}
      footer={
        <div className="btn-row avatar-editor-actions">
          {hasCustom && (
            <Button variant="ghost" onClick={remove} disabled={busy}>
              <Trash2 size={16} aria-hidden="true" />
              {t('avatarRemove')}
            </Button>
          )}
          <Button variant="primary" onClick={save} loading={busy}>
            <Check size={16} aria-hidden="true" />
            {t('avatarSave')}
          </Button>
        </div>
      }
    >
      <div className="avatar-editor stack">
        <div className="avatar-editor-preview">
          {mode === 'photo' && picked && size ? (
            <button
              type="button"
              className="avatar-crop"
              style={{ width: VIEW, height: VIEW }}
              aria-label={t('avatarCropHint')}
              onPointerDown={onPointerDown}
              onPointerMove={onPointerMove}
              onPointerUp={onPointerUp}
              onPointerCancel={onPointerUp}
              onKeyDown={onKeyDown}
            >
              <img
                src={picked.url}
                alt=""
                draggable={false}
                style={{
                  width: size.w,
                  height: size.h,
                  transform: `translate(calc(-50% + ${crop.x}px), calc(-50% + ${crop.y}px))`,
                }}
              />
            </button>
          ) : (
            <Avatar
              name={name}
              size="xl"
              color={color}
              fileId={mode === 'photo' ? current.photo_file_id : null}
              icon={mode === 'icon' || mode === 'animated' ? icon : null}
            />
          )}
        </div>

        <Tabs
          className="avatar-mode"
          ariaLabel={t('avatarTitle')}
          value={mode}
          onChange={(v) => {
            setLocalErr(null);
            const nextMode = v as Mode;
            setMode(nextMode);
            if (nextMode === 'animated' && !isAnimatedAvatar(icon)) {
              setIcon('grok');
            }
          }}
          items={[
            { value: 'photo', label: t('avatarPhoto') },
            { value: 'animated', label: t('avatarAnimated') },
            { value: 'initials', label: t('avatarInitials') },
            { value: 'icon', label: t('avatarIcon') },
          ]}
        />

        {mode === 'photo' && (
          <div className="stack-sm">
            <input
              ref={fileRef}
              type="file"
              accept="image/*"
              className="sr-only"
              aria-label={t('avatarChoosePhoto')}
              onChange={(e) => {
                void pick(e.target.files?.[0]);
                e.target.value = '';
              }}
            />
            <button type="button" className="avatar-drop" onClick={() => fileRef.current?.click()}>
              <ImagePlus size={22} aria-hidden="true" />
              <span className="bold">{picked || current.photo_file_id ? t('avatarChangePhoto') : t('avatarChoosePhoto')}</span>
              <span className="muted tiny">
                {picked?.file?.type === 'image/gif' ? t('avatarGifHint') : t('avatarPhotoHint')}
              </span>
            </button>
            {picked && picked.file?.type !== 'image/gif' && (
              <label className="avatar-zoom">
                <ZoomOut size={18} aria-hidden="true" />
                <input
                  type="range"
                  min={1}
                  max={3}
                  step={0.01}
                  value={crop.zoom}
                  aria-label={t('avatarZoom')}
                  onChange={(e) => setOffset(Number(e.target.value), crop.x, crop.y)}
                />
                <ZoomIn size={18} aria-hidden="true" />
              </label>
            )}
          </div>
        )}

        {mode === 'animated' && (
          <div className="stack-xs">
            <div className="bold small">{t('avatarAnimatedDesc')}</div>
            <div className="avatar-animated-grid" role="radiogroup" aria-label={t('avatarAnimated')}>
              {ANIMATED_AVATARS.map((anim) => {
                const IconComp = AVATAR_ICONS[anim.key];
                const isSelected = icon === anim.key;
                return (
                  <button
                    key={anim.key}
                    type="button"
                    role="radio"
                    aria-checked={isSelected}
                    aria-label={t(`avatarIcon_${anim.key}`)}
                    className={`avatar-animated-card${isSelected ? ' on' : ''}`}
                    onClick={() => setIcon(anim.key)}
                  >
                    <div style={{ width: 44, height: 44 }}>
                      {IconComp && <IconComp size={44} className="avatar-icon" />}
                    </div>
                    <span className="bold small">{t(`avatarIcon_${anim.key}`)}</span>
                  </button>
                );
              })}
            </div>
          </div>
        )}

        {mode === 'icon' && (
          <div className="stack-xs">
            <div className="bold small">{t('avatarPickIconLabel')}</div>
            <div className="avatar-icon-grid" role="radiogroup" aria-label={t('avatarPickIconLabel')}>
              {Object.entries(AVATAR_ICONS)
                .filter(([key]) => !isAnimatedAvatar(key))
                .map(([key, Icon]) => (
                  <button
                    key={key}
                    type="button"
                    role="radio"
                    aria-checked={icon === key}
                    aria-label={t(`avatarIcon_${key}`)}
                    className={`avatar-icon-opt${icon === key ? ' on' : ''}`}
                    onClick={() => setIcon(key)}
                  >
                    <Icon size={20} aria-hidden="true" />
                  </button>
                ))}
            </div>
          </div>
        )}

        {mode !== 'photo' && (
          <div className="stack-xs">
            <div className="bold small">{t('avatarColor')}</div>
            <div className="avatar-swatches" role="radiogroup" aria-label={t('avatarColor')}>
              <button
                type="button"
                role="radio"
                aria-checked={color === null}
                aria-label={t('avatarColorDefault')}
                className={`avatar-swatch av-none${color === null ? ' on' : ''}`}
                onClick={() => setColor(null)}
              />
              {AVATAR_COLORS.map((c) => (
                <button
                  key={c}
                  type="button"
                  role="radio"
                  aria-checked={color === c}
                  aria-label={t(`avatarColor_${c}`)}
                  className={`avatar-swatch av-${c}${color === c ? ' on' : ''}`}
                  onClick={() => setColor(c)}
                />
              ))}
            </div>
          </div>
        )}

        <ErrorBox error={localErr ?? error} />
      </div>
    </Modal>
  );
}
