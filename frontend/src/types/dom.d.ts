export {};

declare global {
  interface DetectedBarcode {
    readonly rawValue: string;
    readonly format: string;
  }

  interface BarcodeDetectorOptions {
    formats?: string[];
  }

  class BarcodeDetector {
    constructor(options?: BarcodeDetectorOptions);
    detect(source: ImageBitmapSource): Promise<DetectedBarcode[]>;
    static getSupportedFormats(): Promise<string[]>;
  }

  interface Window {
    BarcodeDetector?: typeof BarcodeDetector;
  }

  interface HTMLElement {
    inert: boolean;
  }

  // Vite env variable types. These augment the global ImportMetaEnv / ImportMeta
  // declarations that Vite injects, adding Grace Ride–specific VITE_* variables.
  interface ImportMetaEnv {
    /** Sentry DSN for error monitoring. Leave blank or unset to disable. */
    readonly VITE_SENTRY_DSN?: string;
    /** App version injected at build time (e.g. from package.json version). */
    readonly VITE_APP_VERSION?: string;
  }

  interface ImportMeta {
    readonly env: ImportMetaEnv;
  }
}
