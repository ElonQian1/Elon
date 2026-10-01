import { webSourceUrl, type SourceLink } from './sourceLink'
import { decodeImage } from '../../../../../shared/scan/browserScanner.mjs'
export async function scanImageLinks(blob: Blob): Promise<SourceLink[]> {
  if (!blob.type.startsWith('image/') || blob.size > 12 * 1024 * 1024) return []
  const values = await decodeImage(blob, () => new Worker(new URL('./qrWorker.ts', import.meta.url), { type: 'module' }))
  return values.filter(value => !!webSourceUrl(value)).map(url => ({ version: 1, url, method: 'qr' }))
}
