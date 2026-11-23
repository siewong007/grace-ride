import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BellRing, CheckCircle2, Download, Monitor, PlusSquare, Share, Smartphone } from 'lucide-react';
import { usePwaInstall } from '../pwaInstall';
import { enablePushNotifications, usePushNotifications } from '../pushNotifications';
import { Button, Modal, Notice } from './ui';

export interface PwaInstallModalProps {
  onClose: () => void;
  onNotificationsEnabled?: () => void;
}

export function PwaInstallModal({ onClose, onNotificationsEnabled }: PwaInstallModalProps) {
  const { t } = useTranslation('profile');
  const pwa = usePwaInstall();
  const push = usePushNotifications();
  const [installing, setInstalling] = useState(false);
  const [enablingPush, setEnablingPush] = useState(false);
  const [pushFeedback, setPushFeedback] = useState('');

  const handleInstallOneClick = async () => {
    setInstalling(true);
    try {
      const outcome = await pwa.promptInstall();
      if (outcome === 'accepted') {
        onClose();
      }
    } finally {
      setInstalling(false);
    }
  };

  const deniedCopy =
    push.platform === 'ios'
      ? t('pushDeniedIos')
      : push.platform === 'android'
        ? t('pushDeniedAndroid')
        : t('pushDeniedDesktop');

  const handleEnablePush = async () => {
    setEnablingPush(true);
    setPushFeedback('');
    try {
      const res = await enablePushNotifications();
      if (res.ok) {
        setPushFeedback(t(res.mode === 'push' ? 'pushEnabled' : 'pushEnabledLocal'));
        onNotificationsEnabled?.();
      } else if (res.reason === 'denied') {
        setPushFeedback(deniedCopy);
      } else if (res.reason === 'dismissed') {
        setPushFeedback(t('pushDismissed'));
      } else if (res.reason === 'unsupported') {
        setPushFeedback(t('pushUnsupported'));
      }
    } finally {
      setEnablingPush(false);
    }
  };

  return (
    <Modal
      title={t('installAppGuideTitle')}
      onClose={onClose}
      size="md"
      footer={
        <div className="btn-row between wrap">
          <Button variant="secondary" onClick={onClose}>
            {t('close')}
          </Button>
          {!pwa.isInstalled && pwa.canPrompt && (
            <Button
              variant="primary"
              loading={installing}
              onClick={handleInstallOneClick}
            >
              <Download size={16} aria-hidden="true" />
              {t('installAppOneClick')}
            </Button>
          )}
        </div>
      }
    >
      <div className="stack">
        {pwa.isInstalled ? (
          <>
            <div className="empty stack align-center text-center p-md">
              <CheckCircle2 size={48} style={{ color: 'var(--success)' }} aria-hidden="true" />
              <h3 className="bold">{t('installAppInstalled')}</h3>
              <p className="muted small">{t('installAppInstalledDesc')}</p>
            </div>
            {(push.availability === 'prompt' ||
              (push.availability === 'granted' && push.isSubscribed === false)) && (
              <div className="pwa-step2-card">
                <div className="bold row align-center gap-xs">
                  <BellRing size={16} style={{ color: 'var(--primary)' }} aria-hidden="true" />
                  <span>{t('installAppStep2NotifTitle')}</span>
                </div>
                <p className="muted small">{t('installAppStep2NotifDesc')}</p>
                <div>
                  <Button
                    variant="primary"
                    size="sm"
                    loading={enablingPush}
                    onClick={handleEnablePush}
                  >
                    <BellRing size={15} aria-hidden="true" />
                    {t('enableNotifications')}
                  </Button>
                </div>
                {pushFeedback && (
                  <p className="muted tiny" role="status">
                    {pushFeedback}
                  </p>
                )}
              </div>
            )}
            {push.availability === 'granted' && push.isSubscribed !== false && (
              <Notice tone="success" icon={<CheckCircle2 size={16} aria-hidden="true" />}>
                {t('installAppStep2NotifDone')}
              </Notice>
            )}
            {push.availability === 'denied' && (
              <Notice tone="warning">{deniedCopy}</Notice>
            )}
            {push.availability === 'unsupported' && pushFeedback && (
              <p className="muted tiny" role="status">
                {pushFeedback}
              </p>
            )}
          </>
        ) : (
          <>
            {pwa.isInApp && (
              <Notice tone="warning">
                <strong>{t('installAppInAppNotice')}</strong>
              </Notice>
            )}

            {pwa.canPrompt && (
              <div className="card stack-xs" style={{ background: 'var(--surface-2)', padding: 'var(--space-3)' }}>
                <div className="row between align-center wrap gap-xs">
                  <div>
                    <div className="bold">{t('installAppOneClick')}</div>
                    <div className="muted tiny">{t('installAppOneClickPrompt')}</div>
                  </div>
                  <Button
                    variant="primary"
                    size="sm"
                    loading={installing}
                    onClick={handleInstallOneClick}
                  >
                    <Download size={15} aria-hidden="true" />
                    {t('installAppOneClick')}
                  </Button>
                </div>
              </div>
            )}

            {pwa.platform === 'ios' ? (
              <div className="stack-sm">
                <div className="bold small row align-center gap-xs">
                  <Smartphone size={16} aria-hidden="true" />
                  <span>iPhone / iPad (Safari)</span>
                </div>
                <div className="pwa-guide-steps">
                  <div className="pwa-guide-step">
                    <span className="pwa-guide-step-num">1</span>
                    <div className="pwa-guide-step-content">
                      <div className="row align-center gap-xs">
                        <Share size={18} style={{ color: 'var(--primary)' }} aria-hidden="true" />
                        <span>{t('installAppIosStep1')}</span>
                      </div>
                    </div>
                  </div>

                  <div className="pwa-guide-step">
                    <span className="pwa-guide-step-num">2</span>
                    <div className="pwa-guide-step-content">
                      <div className="row align-center gap-xs">
                        <PlusSquare size={18} style={{ color: 'var(--primary)' }} aria-hidden="true" />
                        <span>{t('installAppIosStep2')}</span>
                      </div>
                    </div>
                  </div>

                  <div className="pwa-guide-step">
                    <span className="pwa-guide-step-num">3</span>
                    <div className="pwa-guide-step-content">
                      <span>{t('installAppIosStep3')}</span>
                    </div>
                  </div>

                  <div className="pwa-guide-step">
                    <span className="pwa-guide-step-num">4</span>
                    <div className="pwa-guide-step-content">
                      <span>{t('installAppIosStep4')}</span>
                    </div>
                  </div>
                </div>
              </div>
            ) : pwa.platform === 'android' ? (
              <div className="stack-sm">
                <div className="bold small row align-center gap-xs">
                  <Smartphone size={16} aria-hidden="true" />
                  <span>Android (Chrome)</span>
                </div>
                {pwa.canPrompt && <div className="muted tiny">{t('orManualAdd')}</div>}
                <div className="pwa-guide-steps">
                  <div className="pwa-guide-step">
                    <span className="pwa-guide-step-num">1</span>
                    <div className="pwa-guide-step-content">
                      <span>{t('installAppAndroidStep1')}</span>
                    </div>
                  </div>

                  <div className="pwa-guide-step">
                    <span className="pwa-guide-step-num">2</span>
                    <div className="pwa-guide-step-content">
                      <span>{t('installAppAndroidStep2')}</span>
                    </div>
                  </div>

                  <div className="pwa-guide-step">
                    <span className="pwa-guide-step-num">3</span>
                    <div className="pwa-guide-step-content">
                      <span>{t('installAppAndroidStep3')}</span>
                    </div>
                  </div>

                  <div className="pwa-guide-step">
                    <span className="pwa-guide-step-num">4</span>
                    <div className="pwa-guide-step-content">
                      <span>{t('installAppAndroidStep4')}</span>
                    </div>
                  </div>
                </div>
              </div>
            ) : (
              <div className="stack-sm">
                <div className="bold small row align-center gap-xs">
                  <Monitor size={16} aria-hidden="true" />
                  <span>Desktop / Other</span>
                </div>
                <div className="pwa-guide-steps">
                  <div className="pwa-guide-step">
                    <span className="pwa-guide-step-num">1</span>
                    <div className="pwa-guide-step-content">
                      <span>{t('installAppDesktopStep1')}</span>
                    </div>
                  </div>

                  <div className="pwa-guide-step">
                    <span className="pwa-guide-step-num">2</span>
                    <div className="pwa-guide-step-content">
                      <span>{t('installAppDesktopStep2')}</span>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </Modal>
  );
}
