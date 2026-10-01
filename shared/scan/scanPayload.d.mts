export type ScanKind = 'friend' | 'url' | 'phone' | 'sms' | 'email' | 'geo' | 'wifi' | 'contact' | 'text';
export interface ScanPayload { raw: string; kind: ScanKind; title: string; target: string }
export const MAX_SCAN_LENGTH: number;
export function parseScanPayload(value: unknown): ScanPayload;
export function friendQrPayload(id: string): string;
