import {
  CSSProperties,
  KeyboardEvent as ReactKeyboardEvent,
  PointerEvent as ReactPointerEvent,
  useEffect,
  useRef,
  useState,
} from 'react';
import { Link, useInRouterContext } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import '../styles/grokBotStudio.css';
import { Check, Download, ExternalLink, ImagePlus, Shuffle, Trash2, ZoomIn, ZoomOut } from 'lucide-react';
import { put, uploadFile } from '../api';
import {
  ANIMATED_AVATARS,
  AVATAR_BG_SHAPES,
  AVATAR_BG_SHAPE_PATHS,
  AVATAR_COLORS,
  AVATAR_SHAPES,
  AVATAR_SHAPE_PATHS,
  type AvatarBgShape,
  type AvatarColor,
  avatarGifUrl,
  avatarIconClass,
  type AvatarFields,
  type AvatarShape,
  DEFAULT_ANIMATED_AVATAR,
  encodeAvatarColor,
  encodeAvatarIcon,
  encodeGrokBotIcon,
  getAnimatedAvatarMeta,
  type GrokBotSpec,
  GrokBotSvg,
  isAnimatedAvatar,
  isGrokBotIcon,
  isShapeableAvatar,
  MuseBotSvg,
  parseAvatarColor,
  parseAvatarIcon,
  parseGrokBotIcon,
  STATIC_AVATAR_ICON_KEYS,
} from '../avatarStyle';
import { clampOffset, coverSize, cropToFile, loadImage } from '../cropImage';
import {
  buildStandaloneGrokBotSvg,
  DEFAULT_GROKBOT_SPEC,
  downloadBlob,
  GROKBOT_COLOR_PALETTES,
  grokBotFilename,
  type GrokBotMove,
  randomGrokBotSpec,
  renderGrokBotPngBlob,
} from '../grokBotStudio';
import GrokBotControls from './GrokBotControls';
import GrokBotStudioSvg, { GrokBotStageBackdrop } from './GrokBotSvg';
import { Avatar, Button, ErrorBox, Modal, Tabs, useAction } from './ui';

type Mode = 'animated' | 'studio' | 'photo' | 'initials' | 'icon';

/** Side of the crop circle, in CSS pixels. */
const VIEW = 240;

/** Pick how your avatar looks: coloured animated Grace Ride characters, Grok Bot Studio, a cropped photo, your initial, or an icon. */
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
  const { t: ts } = useTranslation('studio');
  const inRouter = useInRouterContext();
  const { busy, error, run } = useAction();
  const initialBg = parseAvatarColor(current.avatar_color);
  const initialGrokBot = parseGrokBotIcon(current.avatar_icon);
  const initialIcon = parseAvatarIcon(initialGrokBot ? null : current.avatar_icon);
  const hasAnyAvatar = !!(current.photo_file_id || current.avatar_icon || current.avatar_color);
  const [mode, setMode] = useState<Mode>(
    current.photo_file_id
      ? 'photo'
      : isGrokBotIcon(current.avatar_icon)
        ? 'studio'
        : initialIcon.icon === 'initials'
          ? 'initials'
          : isAnimatedAvatar(initialIcon.icon)
            ? 'animated'
            : initialIcon.icon
              ? 'icon'
              : initialBg.bgColor || initialBg.bgShape !== 'circle'
                ? 'initials'
                : 'animated',
  );
  const [studioSpec, setStudioSpec] = useState<GrokBotSpec>(initialGrokBot ?? DEFAULT_GROKBOT_SPEC);
  const [studioMove, setStudioMove] = useState<GrokBotMove>('idle');
  // Bumped on every Move click so picking the same move again replays it.
  const [studioPlayKey, setStudioPlayKey] = useState(0);
  const playStudioMove = (m: GrokBotMove) => {
    setStudioMove(m);
    setStudioPlayKey((k) => k + 1);
  };
  const [color, setColor] = useState<AvatarColor | null>(initialBg.bgColor);
  const [bgShape, setBgShape] = useState<AvatarBgShape>(initialBg.bgShape);
  const [explicitDefault, setExplicitDefault] = useState(false);
  const [icon, setIcon] = useState<string | null>(
    initialIcon.icon === 'initials'
      ? null
      : (initialIcon.icon ?? (!hasAnyAvatar ? DEFAULT_ANIMATED_AVATAR : null)),
  );
  const [avatarShape, setAvatarShape] = useState<AvatarShape | null>(initialIcon.avatarShape);
  const [fgColor, setFgColor] = useState<AvatarColor | null>(initialIcon.fgColor);
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

  const effectiveColor: AvatarColor | null =
    mode === 'animated'
      ? (color ?? (explicitDefault ? null : (getAnimatedAvatarMeta(icon)?.defaultColor ?? null)))
      : color;
  const colorField = encodeAvatarColor(effectiveColor, bgShape);
  const encodedIcon =
    mode === 'studio'
      ? encodeGrokBotIcon(studioSpec)
      : mode === 'initials'
        ? encodeAvatarIcon(null, null, fgColor)
        : encodeAvatarIcon(icon, avatarShape, fgColor);

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
      } else if (mode === 'studio') {
        await put('/me', { clear_photo: true, avatar_icon: encodeGrokBotIcon(studioSpec), avatar_color: '' });
      } else if (mode === 'icon' || mode === 'animated') {
        if (!icon) throw new Error(t('avatarPickIcon'));
        await put('/me', { clear_photo: true, avatar_icon: encodedIcon, avatar_color: colorField });
      } else {
        await put('/me', { clear_photo: true, avatar_icon: encodedIcon, avatar_color: colorField });
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

  const handleDownloadStudioPng = async () => {
    try {
      setLocalErr(null);
      const blob = await renderGrokBotPngBlob(studioSpec, 512);
      downloadBlob(blob, grokBotFilename(studioSpec, 'png'));
    } catch {
      setLocalErr(ts('copyFailed'));
    }
  };

  const handleDownloadStudioSvg = () => {
    const svg = buildStandaloneGrokBotSvg(studioSpec, 512);
    const blob = new Blob([svg], { type: 'image/svg+xml;charset=utf-8' });
    downloadBlob(blob, grokBotFilename(studioSpec, 'svg'));
  };

  const hasCustom = !!(current.photo_file_id || current.avatar_icon || current.avatar_color);
  const size = picked ? coverSize(picked.img.naturalWidth, picked.img.naturalHeight, VIEW, crop.zoom) : null;
  const studioReadout = `${ts(`colors.${studioSpec.color}`)} ${ts(`shapes.${studioSpec.shape}`)} · ${ts(`faces.${studioSpec.face}`)}`.toUpperCase();
  const studioGlowVars = {
    '--grok-glow': GROKBOT_COLOR_PALETTES[studioSpec.color].glow,
  } as CSSProperties;

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
          ) : mode === 'studio' ? (
            <div className="grok-studio-mini-stage" style={studioGlowVars}>
              <div className="grok-studio-stage-topbar">
                <span className="grok-studio-status">{studioReadout}</span>
                <button
                  type="button"
                  className="grok-studio-surprise-btn"
                  onClick={() => setStudioSpec(randomGrokBotSpec(studioSpec))}
                >
                  <Shuffle size={13} aria-hidden="true" />
                  <span>{ts('surpriseMe')}</span>
                </button>
              </div>
              <div className="grok-studio-stage compact">
                <GrokBotStageBackdrop color={studioSpec.color} />
                <GrokBotStudioSvg
                  shape={studioSpec.shape}
                  face={studioSpec.face}
                  color={studioSpec.color}
                  move={studioMove}
                  playKey={studioPlayKey}
                  variant="stage"
                  trackCursor
                  className="grok-studio-stage-svg"
                />
                <div className="grok-studio-cursor-hint">{ts('cursorHint')}</div>
              </div>
            </div>
          ) : (
            <Avatar
              name={name}
              size="xl"
              color={colorField}
              fileId={mode === 'photo' ? current.photo_file_id : null}
              icon={mode === 'photo' ? null : encodedIcon}
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
              setIcon(DEFAULT_ANIMATED_AVATAR);
              setAvatarShape(null);
            } else if (nextMode === 'icon' && isAnimatedAvatar(icon)) {
              setIcon(null);
              setAvatarShape(null);
            } else if (nextMode === 'initials') {
              setAvatarShape(null);
            }
          }}
          items={[
            { value: 'animated', label: t('avatarAnimated') },
            { value: 'studio', label: t('avatarStudio') },
            { value: 'photo', label: t('avatarPhoto') },
            { value: 'initials', label: t('avatarInitials') },
            { value: 'icon', label: t('avatarIcon') },
          ]}
        />

        {mode === 'studio' && (
          <div className="stack-sm">
            <GrokBotControls
              spec={studioSpec}
              onSpecChange={setStudioSpec}
              move={studioMove}
              onMoveChange={playStudioMove}
              compact
            />
            <div className="grok-studio-mini-footer">
              <div className="btn-row">
                <button
                  type="button"
                  className="btn secondary sm"
                  onClick={() => void handleDownloadStudioPng()}
                >
                  <Download size={14} aria-hidden="true" />
                  <span>{ts('downloadPng')}</span>
                </button>
                <button
                  type="button"
                  className="btn secondary sm"
                  onClick={handleDownloadStudioSvg}
                >
                  <Download size={14} aria-hidden="true" />
                  <span>{ts('downloadSvg')}</span>
                </button>
              </div>
              {inRouter ? (
                <Link
                  to={`/studio?shape=${studioSpec.shape}&color=${studioSpec.color}&face=${studioSpec.face}`}
                  className="grok-studio-open-link"
                  onClick={onClose}
                >
                  <span>{ts('openFullStudio')}</span>
                  <ExternalLink size={14} aria-hidden="true" />
                </Link>
              ) : (
                <a
                  href={`/studio?shape=${studioSpec.shape}&color=${studioSpec.color}&face=${studioSpec.face}`}
                  className="grok-studio-open-link"
                  onClick={onClose}
                >
                  <span>{ts('openFullStudio')}</span>
                  <ExternalLink size={14} aria-hidden="true" />
                </a>
              )}
            </div>
          </div>
        )}

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
                const isSelected = icon === anim.key;
                const thumbColor = isSelected && effectiveColor ? effectiveColor : anim.defaultColor;
                const thumbFg = isSelected && fgColor ? fgColor : null;
                return (
                  <button
                    key={anim.key}
                    type="button"
                    role="radio"
                    aria-checked={isSelected}
                    aria-label={t(`avatarIcon_${anim.key}`)}
                    className={`avatar-animated-card${isSelected ? ' on' : ''}`}
                    onClick={() => {
                      setIcon(anim.key);
                      setAvatarShape(null);
                      if (color === null && !explicitDefault) {
                        setColor(anim.defaultColor);
                      }
                    }}
                  >
                    <div
                      className={`avatar av-${thumbColor}${thumbFg ? ` av-fg-${thumbFg}` : ''} avatar-animated avatar-animated-thumb`}
                      aria-hidden="true"
                    >
                      {anim.key === 'grok' && isSelected && avatarShape && avatarShape !== 'hexagon' ? (
                        <GrokBotSvg
                          shape={avatarShape}
                          className="avatar-icon"
                        />
                      ) : anim.key === 'muse' && isSelected && avatarShape && avatarShape !== 'cloud' ? (
                        <MuseBotSvg
                          shape={avatarShape}
                          className="avatar-icon"
                        />
                      ) : (
                        <img
                          src={avatarGifUrl(anim.key, thumbFg ?? thumbColor)}
                          alt=""
                          className={`avatar-animated-icon ${anim.className} avatar-icon avatar-gif`}
                          decoding="async"
                        />
                      )}
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
              {STATIC_AVATAR_ICON_KEYS.map((key) => (
                <button
                  key={key}
                  type="button"
                  role="radio"
                  aria-checked={icon === key}
                  aria-label={t(`avatarIcon_${key}`)}
                  className={`avatar-icon-opt${icon === key ? ' on' : ''}`}
                  onClick={() => setIcon(key)}
                >
                  <img
                    src={avatarGifUrl(key, effectiveColor)}
                    alt=""
                    width={28}
                    height={28}
                    className={`avatar-animated-icon ${avatarIconClass(key)} avatar-icon-gif`}
                    aria-hidden="true"
                    decoding="async"
                  />
                </button>
              ))}
            </div>
          </div>
        )}

        {mode === 'animated' && isShapeableAvatar(icon) && (
          <div className="stack-xs">
            <div className="bold small">{t('avatarShape')}</div>
            <div className="avatar-shape-grid" role="radiogroup" aria-label={t('avatarShape')}>
              {AVATAR_SHAPES.map((s) => {
                const currentShape = avatarShape ?? (icon === 'muse' ? 'cloud' : 'hexagon');
                const isSel = currentShape === s;
                return (
                  <button
                    key={s}
                    type="button"
                    role="radio"
                    aria-checked={isSel}
                    aria-label={t(`avatarShape_${s}`)}
                    title={t(`avatarShape_${s}`)}
                    className={`avatar-shape-opt${isSel ? ' on' : ''}`}
                    onClick={() => setAvatarShape(s)}
                  >
                    <svg viewBox="0 0 48 48" width="22" height="22" fill="currentColor" aria-hidden="true">
                      <path d={AVATAR_SHAPE_PATHS[s]} />
                    </svg>
                  </button>
                );
              })}
            </div>
          </div>
        )}

        {(mode === 'initials' || (mode === 'animated' && isShapeableAvatar(icon))) && (
          <div className="stack-xs">
            <div className="bold small">{t('avatarFgColor')}</div>
            <div className="avatar-swatches" role="radiogroup" aria-label={t('avatarFgColor')}>
              <button
                type="button"
                role="radio"
                aria-checked={fgColor === null}
                aria-label={`${t('avatarFgColor')}: ${t('avatarColorAuto')}`}
                title={t('avatarColorAuto')}
                className={`avatar-swatch av-none${fgColor === null ? ' on' : ''}`}
                onClick={() => setFgColor(null)}
              />
              {AVATAR_COLORS.map((c) => (
                <button
                  key={c}
                  type="button"
                  role="radio"
                  aria-checked={fgColor === c}
                  aria-label={`${t('avatarFgColor')}: ${t(`avatarColor_${c}`)}`}
                  title={t(`avatarColor_${c}`)}
                  className={`avatar-swatch av-fg-swatch av-fg-${c}${fgColor === c ? ' on' : ''}`}
                  onClick={() => setFgColor(c)}
                />
              ))}
            </div>
          </div>
        )}

        {mode !== 'studio' && (
          <div className="stack-xs">
            <div className="bold small">{t('avatarBgShape')}</div>
            <div className="avatar-shape-grid" role="radiogroup" aria-label={t('avatarBgShape')}>
              {AVATAR_BG_SHAPES.map((s) => {
                const isSel = bgShape === s;
                return (
                  <button
                    key={s}
                    type="button"
                    role="radio"
                    aria-checked={isSel}
                    aria-label={t(`avatarBgShape_${s}`)}
                    title={t(`avatarBgShape_${s}`)}
                    className={`avatar-shape-opt${isSel ? ' on' : ''}`}
                    onClick={() => setBgShape(s)}
                  >
                    {s === 'hexagon' || s === 'cloud' ? (
                      <svg viewBox="0 0 48 48" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="3.4" aria-hidden="true">
                        <path d={AVATAR_BG_SHAPE_PATHS[s]} />
                      </svg>
                    ) : (
                      <span className={`avatar-bg-mini av-bg-${s}`} aria-hidden="true" />
                    )}
                  </button>
                );
              })}
            </div>
          </div>
        )}

        {mode !== 'photo' && mode !== 'studio' && (
          <div className="stack-xs">
            <div className="bold small">{t('avatarColor')}</div>
            <div className="avatar-swatches" role="radiogroup" aria-label={t('avatarColor')}>
              <button
                type="button"
                role="radio"
                aria-checked={effectiveColor === null}
                aria-label={t('avatarColorDefault')}
                className={`avatar-swatch av-none${effectiveColor === null ? ' on' : ''}`}
                onClick={() => {
                  setColor(null);
                  setExplicitDefault(true);
                }}
              />
              {AVATAR_COLORS.map((c) => (
                <button
                  key={c}
                  type="button"
                  role="radio"
                  aria-checked={effectiveColor === c}
                  aria-label={t(`avatarColor_${c}`)}
                  className={`avatar-swatch av-${c}${effectiveColor === c ? ' on' : ''}`}
                  onClick={() => {
                    setColor(c);
                    setExplicitDefault(false);
                  }}
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
