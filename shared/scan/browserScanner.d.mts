export const MAX_IMAGE_BYTES: number;
export type WorkerFactory = () => Worker;
export function decodeFrame(frame: ImageData, workerFactory: WorkerFactory, signal?: AbortSignal): Promise<string[]>;
export function decodeImage(blob: Blob, workerFactory: WorkerFactory, signal?: AbortSignal): Promise<string[]>;
export function fetchScanImage(url: string, signal?: AbortSignal): Promise<Blob>;
export function cameraErrorMessage(error: unknown): string;
export class ScanCamera {
  constructor(video: HTMLVideoElement, workerFactory: WorkerFactory, onResults: (values: string[]) => void, onState: (message: string) => void);
  facing: string;
  stop(): void;
  start(facing?: string): Promise<void>;
  canTorch(): boolean;
  torch(enabled: boolean): Promise<void>;
}
