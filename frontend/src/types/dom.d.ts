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
}
