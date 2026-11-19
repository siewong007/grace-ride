import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { CheckCircle2, Download, Monitor, PlusSquare, Share, Smartphone } from 'lucide-react';
import { usePwaInstall } from '../pwaInstall';
import { Button, Modal, Notice } from './ui';

export interface PwaInstallModalProps {
  onClose: () => void;
}

export function PwaInstallModal({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation('profile');
  const pwa = usePwaInstall();
  const [installing, setInstalling] = useState(false);

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
          <div className="empty stack align-center text-center p-md">
            <CheckCircle2 size={48} style={{ color: 'var(--success)' }} aria-hidden="true" />
            <h3 className="bold">{t('installAppInstalled')}</h3>
            <p className="muted small">{t('installAppInstalledDesc')}</p>
          </div>
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
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </Modal>
  );
}
