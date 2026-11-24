import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { AlertCircle } from 'lucide-react';
import jsQR from 'jsqr';
import { useWakeLock } from '../../hooks/useWakeLock';

export interface QrCameraScannerProps {
  onScan: (decodedText: string) => void;
  active?: boolean;
  hint?: string;
  className?: string;
  style?: React.CSSProperties;
}

export function QrCameraScanner({
  onScan,
  active = true,
  hint,
  className,
  style,
}: QrCameraScannerProps) {
  const { t } = useTranslation('eventDetail');
  const [cameraError, setCameraError] = useState<string | null>(null);
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const timerRef = useRef<number | null>(null);
  const isScanningRef = useRef<boolean>(false);
  const onScanRef = useRef(onScan);
  onScanRef.current = onScan;
  useWakeLock(active && !cameraError);

  useEffect(() => {
    const stopCamera = () => {
      if (timerRef.current) {
        window.clearTimeout(timerRef.current);
        timerRef.current = null;
      }
      if (streamRef.current) {
        streamRef.current.getTracks().forEach((track) => track.stop());
        streamRef.current = null;
      }
      if (videoRef.current) {
        videoRef.current.srcObject = null;
      }
    };

    if (!active) {
      stopCamera();
      return;
    }

    let cancelled = false;
    isScanningRef.current = true;
    setCameraError(null);

    const startCamera = async () => {
      if (!navigator.mediaDevices?.getUserMedia) {
        setCameraError(t('cameraUnavailable'));
        return;
      }

      try {
        const stream = await navigator.mediaDevices.getUserMedia({
          video: {
            facingMode: 'environment',
            width: { ideal: 1280 },
            height: { ideal: 720 },
          },
        });

        if (cancelled) {
          stream.getTracks().forEach((track) => track.stop());
          return;
        }

        streamRef.current = stream;
        if (videoRef.current) {
          videoRef.current.srcObject = stream;
          await videoRef.current.play().catch(() => {});
        }

        // Initialize native BarcodeDetector if available
        let barcodeDetector: { detect: (src: HTMLVideoElement) => Promise<{ rawValue?: string }[]> } | null = null;
        if (typeof window !== 'undefined' && 'BarcodeDetector' in window) {
          try {
            // @ts-expect-error Standard BarcodeDetector in modern browsers
            barcodeDetector = new window.BarcodeDetector({ formats: ['qr_code'] });
          } catch {
            barcodeDetector = null;
          }
        }

        const scanLoop = async () => {
          if (cancelled || !isScanningRef.current) return;
          const video = videoRef.current;
          if (video && video.readyState >= HTMLMediaElement.HAVE_ENOUGH_DATA) {
            let foundRaw: string | null = null;

            // 1. Try native BarcodeDetector
            if (barcodeDetector) {
              try {
                const barcodes = await barcodeDetector.detect(video);
                if (barcodes.length > 0 && barcodes[0].rawValue) {
                  foundRaw = barcodes[0].rawValue;
                }
              } catch {
                // fall back to jsQR
              }
            }

            // 2. Fall back to jsQR via offscreen canvas
            if (!foundRaw) {
              try {
                if (!canvasRef.current) {
                  canvasRef.current = document.createElement('canvas');
                }
                const canvas = canvasRef.current;
                const vw = video.videoWidth;
                const vh = video.videoHeight;
                if (vw > 0 && vh > 0) {
                  const scale = Math.min(1, 640 / Math.max(vw, vh));
                  const cw = Math.floor(vw * scale);
                  const ch = Math.floor(vh * scale);
                  canvas.width = cw;
                  canvas.height = ch;
                  const ctx = canvas.getContext('2d', { willReadFrequently: true });
                  if (ctx) {
                    ctx.drawImage(video, 0, 0, cw, ch);
                    const imgData = ctx.getImageData(0, 0, cw, ch);
                    const qr = jsQR(imgData.data, cw, ch, {
                      inversionAttempts: 'dontInvert',
                    });
                    if (qr && qr.data) {
                      foundRaw = qr.data;
                    }
                  }
                }
              } catch {
                // ignore canvas read errors
              }
            }

            if (foundRaw) {
              try {
                navigator.vibrate?.(80);
              } catch {
                // ignore
              }
              isScanningRef.current = false;
              stopCamera();
              onScanRef.current(foundRaw);
              return;
            }
          }

          if (!cancelled && isScanningRef.current) {
            timerRef.current = window.setTimeout(scanLoop, 200);
          }
        };

        timerRef.current = window.setTimeout(scanLoop, 300);
      } catch {
        if (!cancelled) {
          setCameraError(t('cameraUnavailable'));
        }
      }
    };

    void startCamera();

    return () => {
      cancelled = true;
      isScanningRef.current = false;
      stopCamera();
    };
  }, [active, t]);

  if (cameraError) {
    return (
      <div
        data-testid="qr-camera-error"
        className="stack items-center justify-center text-center p-md"
        style={{
          width: '100%',
          maxWidth: '360px',
          minHeight: '160px',
          background: 'var(--surface-2)',
          borderRadius: 'var(--radius-lg, 16px)',
          border: '1px dashed var(--border)',
          margin: '0 auto',
          padding: '1.5rem',
        }}
      >
        <div style={{ color: 'var(--warning)', marginBottom: '0.5rem' }}>
          <AlertCircle size={36} />
        </div>
        <p className="small muted" style={{ maxWidth: '280px' }}>
          {cameraError}
        </p>
      </div>
    );
  }

  return (
    <div
      data-testid="qr-camera-scanner"
      className={className}
      style={{
        position: 'relative',
        width: '100%',
        maxWidth: '360px',
        margin: '0 auto',
        borderRadius: 'var(--radius-lg, 16px)',
        overflow: 'hidden',
        background: 'black',
        aspectRatio: '1 / 1',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        boxShadow: '0 4px 20px rgba(0, 0, 0, 0.15)',
        ...style,
      }}
    >
      <video
        ref={videoRef}
        playsInline
        muted
        style={{
          width: '100%',
          height: '100%',
          objectFit: 'cover',
          display: 'block',
        }}
      />

      {/* Viewfinder Target Overlay */}
      <div
        style={{
          position: 'absolute',
          inset: 0,
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          pointerEvents: 'none',
        }}
      >
        {/* Reticle / Target Box */}
        <div
          style={{
            position: 'relative',
            width: '68%',
            height: '68%',
            boxShadow: '0 0 0 9999px rgba(0, 0, 0, 0.45)',
            borderRadius: '12px',
          }}
        >
          {/* 4 Corner Markers */}
          <div
            style={{
              position: 'absolute',
              top: 0,
              left: 0,
              width: '24px',
              height: '24px',
              borderTop: '3px solid white',
              borderLeft: '3px solid white',
              borderTopLeftRadius: '10px',
            }}
          />
          <div
            style={{
              position: 'absolute',
              top: 0,
              right: 0,
              width: '24px',
              height: '24px',
              borderTop: '3px solid white',
              borderRight: '3px solid white',
              borderTopRightRadius: '10px',
            }}
          />
          <div
            style={{
              position: 'absolute',
              bottom: 0,
              left: 0,
              width: '24px',
              height: '24px',
              borderBottom: '3px solid white',
              borderLeft: '3px solid white',
              borderBottomLeftRadius: '10px',
            }}
          />
          <div
            style={{
              position: 'absolute',
              bottom: 0,
              right: 0,
              width: '24px',
              height: '24px',
              borderBottom: '3px solid white',
              borderRight: '3px solid white',
              borderBottomRightRadius: '10px',
            }}
          />
          {active && (
            <div className="qr-scan-track" aria-hidden="true">
              <div className="qr-scan-line" />
            </div>
          )}
        </div>

        {/* Viewfinder Guide Caption */}
        <p
          className="tiny bold"
          style={{
            color: 'white',
            marginTop: '1rem',
            textShadow: '0 1px 3px rgba(0,0,0,0.8)',
            letterSpacing: '0.02em',
          }}
        >
          {hint || t('cameraActiveNotice')}
        </p>
      </div>
    </div>
  );
}
