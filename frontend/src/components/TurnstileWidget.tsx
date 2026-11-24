import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from './ui';
import { loadTurnstileScript, type TurnstileAction, type TurnstileApi } from '../turnstile';

export interface TurnstileWidgetProps {
  siteKey: string | null;
  action: TurnstileAction;
  resetKey?: number;
  onToken: (token: string | null) => void;
}

type WidgetStatus = 'loading' | 'ready' | 'expired' | 'error';

function readTheme(): 'light' | 'dark' | 'auto' {
  if (typeof document === 'undefined') return 'auto';
  const t = document.documentElement.dataset.theme;
  return t === 'light' || t === 'dark' ? t : 'auto';
}

export default function TurnstileWidget({
  siteKey,
  action,
  resetKey = 0,
  onToken,
}: TurnstileWidgetProps) {
  const { t, i18n } = useTranslation('common');
  const containerRef = useRef<HTMLDivElement | null>(null);
  const widgetIdRef = useRef<string | null>(null);
  const prevResetKeyRef = useRef(resetKey);
  const onTokenRef = useRef(onToken);
  const [status, setStatus] = useState<WidgetStatus>('loading');
  const [theme, setTheme] = useState<'light' | 'dark' | 'auto'>(readTheme);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    onTokenRef.current = onToken;
  }, [onToken]);

  useEffect(() => {
    if (typeof document === 'undefined' || typeof MutationObserver === 'undefined') return;
    const root = document.documentElement;
    const obs = new MutationObserver(() => setTheme(readTheme()));
    obs.observe(root, { attributes: true, attributeFilter: ['data-theme'] });
    return () => obs.disconnect();
  }, []);

  const turnstileLang = i18n.language?.startsWith('zh') ? 'zh-cn' : 'en';

  useEffect(() => {
    if (!siteKey) return;

    let cancelled = false;
    onTokenRef.current(null);
    setStatus('loading');

    const mount = (api: TurnstileApi) => {
      if (cancelled || !containerRef.current) return;
      containerRef.current.innerHTML = '';
      let callbackFired = false;
      try {
        const id = api.render(containerRef.current, {
          sitekey: siteKey,
          action,
          theme,
          language: turnstileLang,
          size: 'flexible',
          'refresh-expired': 'auto',
          callback: (tok) => {
            if (cancelled) return;
            callbackFired = true;
            setStatus('ready');
            onTokenRef.current(tok);
          },
          'expired-callback': () => {
            if (cancelled) return;
            callbackFired = true;
            setStatus('expired');
            onTokenRef.current(null);
          },
          'error-callback': () => {
            if (cancelled) return;
            callbackFired = true;
            setStatus('error');
            onTokenRef.current(null);
          },
          'timeout-callback': () => {
            if (cancelled) return;
            callbackFired = true;
            setStatus('expired');
            onTokenRef.current(null);
          },
        });
        widgetIdRef.current = id;
        if (!callbackFired) {
          setStatus('ready');
        }
      } catch {
        widgetIdRef.current = null;
        setStatus('error');
        onTokenRef.current(null);
      }
    };

    if (typeof window !== 'undefined' && window.turnstile) {
      mount(window.turnstile);
    } else {
      void loadTurnstileScript()
        .then(mount)
        .catch(() => {
          if (cancelled) return;
          widgetIdRef.current = null;
          setStatus('error');
          onTokenRef.current(null);
        });
    }

    return () => {
      cancelled = true;
      const id = widgetIdRef.current;
      widgetIdRef.current = null;
      if (id && typeof window !== 'undefined') {
        try {
          window.turnstile?.remove(id);
        } catch {
          /* ignore teardown error */
        }
      }
    };
  }, [siteKey, action, theme, turnstileLang, attempt]);

  useEffect(() => {
    if (resetKey === prevResetKeyRef.current) return;
    prevResetKeyRef.current = resetKey;
    onTokenRef.current(null);
    if (!siteKey) return;

    const id = widgetIdRef.current;
    if (id && typeof window !== 'undefined' && window.turnstile) {
      try {
        setStatus('ready');
        window.turnstile.reset(id);
        return;
      } catch {
        /* fall through to full remount */
      }
    }
    setAttempt((a) => a + 1);
  }, [resetKey, siteKey]);

  if (!siteKey) return null;

  const handleRetry = () => {
    onTokenRef.current(null);
    const id = widgetIdRef.current;
    if (id && typeof window !== 'undefined' && window.turnstile) {
      try {
        setStatus('ready');
        window.turnstile.reset(id);
        return;
      } catch {
        /* fall through to full remount */
      }
    }
    setAttempt((a) => a + 1);
  };

  return (
    <div className="turnstile-wrap stack-xs" role="group" aria-label={t('turnstile.label')}>
      {status === 'loading' && (
        <div className="muted small" aria-live="polite">
          {t('turnstile.loading')}
        </div>
      )}
      <div ref={containerRef} className="turnstile-slot" />
      {(status === 'error' || status === 'expired') && (
        <div className="stack-xs" role="alert">
          <div className="field-error">
            {status === 'expired' ? t('turnstile.expired') : t('turnstile.error')}
          </div>
          <div>
            <Button variant="secondary" size="sm" type="button" onClick={handleRetry}>
              {t('turnstile.retry')}
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
