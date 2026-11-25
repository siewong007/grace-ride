import { CSSProperties, useEffect, useRef, useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { Check, ChevronDown, Copy, Download, FileCode2, Image as ImageIcon, Link2, Shuffle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import '../styles/grokBotStudio.css';
import { put } from '../api';
import { useAuth } from '../auth';
import GrokBotControls from '../components/GrokBotControls';
import GrokBotStudioSvg, { GrokBotStageBackdrop } from '../components/GrokBotSvg';
import { Button, ErrorBox, Page, useAction, useToast } from '../components/ui';
import {
  buildStandaloneGrokBotSvg,
  DEFAULT_GROKBOT_SPEC,
  downloadBlob,
  encodeGrokBotIcon,
  GROKBOT_COLOR_PALETTES,
  grokBotFilename,
  type GrokBotMove,
  type GrokBotSpec,
  isGrokBotColor,
  isGrokBotFace,
  isGrokBotShape,
  parseGrokBotIcon,
  randomGrokBotSpec,
  renderGrokBotPngBlob,
} from '../grokBotStudio';

function resolveSpecFromParams(params: URLSearchParams, fallback: GrokBotSpec): GrokBotSpec {
  const s = params.get('shape');
  const c = params.get('color');
  const f = params.get('face');
  return {
    shape: isGrokBotShape(s) ? s : fallback.shape,
    color: isGrokBotColor(c) ? c : fallback.color,
    face: isGrokBotFace(f) ? f : fallback.face,
  };
}

export default function GrokBotStudioPage() {
  const { t } = useTranslation('studio');
  const { me, refresh } = useAuth();
  const toast = useToast();
  const { busy, error, run } = useAction();
  const [searchParams, setSearchParams] = useSearchParams();

  const savedSpec = parseGrokBotIcon(me?.avatar_icon) ?? DEFAULT_GROKBOT_SPEC;
  const spec = resolveSpecFromParams(searchParams, savedSpec);
  const [activeMove, setActiveMove] = useState<GrokBotMove>('idle');
  // Bumped on every Move click so picking the same move again replays it.
  const [playKey, setPlayKey] = useState(0);
  const playMove = (m: GrokBotMove) => {
    setActiveMove(m);
    setPlayKey((k) => k + 1);
  };
  const [exportBusy, setExportBusy] = useState(false);
  const [copyOpen, setCopyOpen] = useState(false);
  const copyWrapRef = useRef<HTMLDivElement>(null);

  const updateSpec = (next: GrokBotSpec) => {
    setSearchParams(
      {
        shape: next.shape,
        color: next.color,
        face: next.face,
      },
      { replace: true },
    );
  };

  useEffect(() => {
    if (!copyOpen) return;
    const onDocClick = (e: MouseEvent) => {
      if (copyWrapRef.current && !copyWrapRef.current.contains(e.target as Node)) {
        setCopyOpen(false);
      }
    };
    const onEsc = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setCopyOpen(false);
    };
    document.addEventListener('mousedown', onDocClick);
    document.addEventListener('keydown', onEsc);
    return () => {
      document.removeEventListener('mousedown', onDocClick);
      document.removeEventListener('keydown', onEsc);
    };
  }, [copyOpen]);

  const handleSurprise = () => {
    updateSpec(randomGrokBotSpec(spec));
  };

  const handleSaveAvatar = () =>
    run(async () => {
      await put('/me', {
        clear_photo: true,
        avatar_icon: encodeGrokBotIcon(spec),
        avatar_color: '',
      });
      await refresh();
    }, t('avatarSaved'));

  const handleDownloadPng = async () => {
    setExportBusy(true);
    try {
      const blob = await renderGrokBotPngBlob(spec, 512);
      downloadBlob(blob, grokBotFilename(spec, 'png'));
    } catch {
      toast.error(t('copyFailed'));
    } finally {
      setExportBusy(false);
    }
  };

  const handleDownloadSvg = () => {
    const svg = buildStandaloneGrokBotSvg(spec, 512);
    const blob = new Blob([svg], { type: 'image/svg+xml;charset=utf-8' });
    downloadBlob(blob, grokBotFilename(spec, 'svg'));
  };

  const handleCopyImage = async () => {
    setCopyOpen(false);
    setExportBusy(true);
    try {
      if (typeof ClipboardItem !== 'undefined' && navigator.clipboard?.write) {
        const blobPromise = renderGrokBotPngBlob(spec, 512);
        try {
          // Pass Promise<Blob> synchronously so Safari/WebKit preserves user activation gesture.
          await navigator.clipboard.write([new ClipboardItem({ 'image/png': blobPromise })]);
        } catch (err) {
          if (err instanceof TypeError) {
            const resolved = await blobPromise;
            await navigator.clipboard.write([new ClipboardItem({ 'image/png': resolved })]);
          } else {
            throw err;
          }
        }
        toast.success(t('copiedImage'));
      } else {
        const blob = await renderGrokBotPngBlob(spec, 512);
        downloadBlob(blob, grokBotFilename(spec, 'png'));
        toast.info(t('downloadPng'));
      }
    } catch {
      toast.error(t('copyFailed'));
    } finally {
      setExportBusy(false);
    }
  };

  const handleCopySvg = async () => {
    setCopyOpen(false);
    try {
      const svg = buildStandaloneGrokBotSvg(spec, 512);
      await navigator.clipboard.writeText(svg);
      toast.success(t('copiedSvg'));
    } catch {
      toast.error(t('copyFailed'));
    }
  };

  const handleCopyLink = async () => {
    setCopyOpen(false);
    try {
      const url = `${window.location.origin}/studio?shape=${spec.shape}&color=${spec.color}&face=${spec.face}`;
      await navigator.clipboard.writeText(url);
      toast.success(t('copiedLink'));
    } catch {
      toast.error(t('copyFailed'));
    }
  };

  const statusReadout = `${t(`colors.${spec.color}`)} ${t(`shapes.${spec.shape}`)} · ${t(`faces.${spec.face}`)}`.toUpperCase();
  const palette = GROKBOT_COLOR_PALETTES[spec.color];
  const stageVars = {
    '--grok-glow': palette.glow,
  } as CSSProperties;

  return (
    <Page className="grok-studio-page stack">
      <header className="grok-studio-hero stack-xs">
        <nav className="grok-studio-breadcrumbs" aria-label="Breadcrumb">
          <Link to="/">{t('breadcrumbHome')}</Link>
          <span aria-hidden="true">/</span>
          <span>{t('breadcrumbStudio')}</span>
        </nav>
        <h1>{t('title')}</h1>
        <p className="muted grok-studio-subtitle">{t('subtitle')}</p>
      </header>

      <div className="grok-studio-layout">
        <div className="grok-studio-stage-card" style={stageVars}>
          <div className="grok-studio-stage-topbar">
            <span className="grok-studio-status" data-testid="studio-status-readout">
              {statusReadout}
            </span>
            <button
              type="button"
              className="grok-studio-surprise-btn"
              onClick={handleSurprise}
            >
              <Shuffle size={14} aria-hidden="true" />
              <span>{t('surpriseMe')}</span>
            </button>
          </div>

          <div className="grok-studio-stage">
            <GrokBotStageBackdrop color={spec.color} />
            <GrokBotStudioSvg
              shape={spec.shape}
              face={spec.face}
              color={spec.color}
              move={activeMove}
              playKey={playKey}
              variant="stage"
              trackCursor
              className="grok-studio-stage-svg"
            />
            <div className="grok-studio-cursor-hint">{t('cursorHint')}</div>
          </div>

          <div className="grok-studio-actions">
            <Button
              variant="primary"
              onClick={handleSaveAvatar}
              loading={busy}
              className="grok-studio-save-btn"
            >
              <Check size={16} aria-hidden="true" />
              <span>{t('saveAsAvatar')}</span>
            </Button>

            <div className="grok-studio-export-row">
              <button
                type="button"
                className="btn secondary sm"
                aria-label={t('downloadPngAria')}
                disabled={exportBusy}
                onClick={() => void handleDownloadPng()}
              >
                <Download size={15} aria-hidden="true" />
                <span>{t('downloadPng')}</span>
              </button>

              <button
                type="button"
                className="btn secondary sm"
                aria-label={t('downloadSvgAria')}
                onClick={handleDownloadSvg}
              >
                <Download size={15} aria-hidden="true" />
                <span>{t('downloadSvg')}</span>
              </button>

              <div className="grok-studio-copy-wrap" ref={copyWrapRef}>
                <button
                  type="button"
                  className="btn secondary sm"
                  aria-haspopup="menu"
                  aria-expanded={copyOpen}
                  onClick={() => setCopyOpen((v) => !v)}
                >
                  <Copy size={15} aria-hidden="true" />
                  <span>{t('copy')}</span>
                  <ChevronDown size={14} aria-hidden="true" />
                </button>
                {copyOpen && (
                  <div className="grok-studio-copy-menu" role="menu">
                    <button
                      type="button"
                      role="menuitem"
                      className="grok-studio-copy-item"
                      onClick={() => void handleCopyImage()}
                    >
                      <ImageIcon size={15} aria-hidden="true" />
                      <span>{t('copyImage')}</span>
                    </button>
                    <button
                      type="button"
                      role="menuitem"
                      className="grok-studio-copy-item"
                      onClick={() => void handleCopySvg()}
                    >
                      <FileCode2 size={15} aria-hidden="true" />
                      <span>{t('copySvg')}</span>
                    </button>
                    <button
                      type="button"
                      role="menuitem"
                      className="grok-studio-copy-item"
                      onClick={() => void handleCopyLink()}
                    >
                      <Link2 size={15} aria-hidden="true" />
                      <span>{t('copyLink')}</span>
                    </button>
                  </div>
                )}
              </div>
            </div>
          </div>

          <ErrorBox error={error} />
        </div>

        <div className="grok-studio-controls-card">
          <GrokBotControls
            spec={spec}
            onSpecChange={updateSpec}
            move={activeMove}
            onMoveChange={playMove}
          />
        </div>
      </div>

      <section className="grok-studio-how stack" aria-labelledby="grok-how-heading">
        <h2 id="grok-how-heading">{t('howTitle')}</h2>
        <div className="grok-studio-steps-grid">
          <div className="grok-studio-step-card">
            <span className="grok-studio-step-num">01</span>
            <h3>{t('step1Title')}</h3>
            <p className="muted small">{t('step1Body')}</p>
          </div>
          <div className="grok-studio-step-card">
            <span className="grok-studio-step-num">02</span>
            <h3>{t('step2Title')}</h3>
            <p className="muted small">{t('step2Body')}</p>
          </div>
          <div className="grok-studio-step-card">
            <span className="grok-studio-step-num">03</span>
            <h3>{t('step3Title')}</h3>
            <p className="muted small">{t('step3Body')}</p>
          </div>
        </div>
      </section>
    </Page>
  );
}
