import type { RecordRow } from './recordApi'
import type { LinkPreview } from '../socialLinks'
declare global {
  var ElonRecordPresentation: { text(row: RecordRow, cards?: Pick<LinkPreview, 'url' | 'title' | 'site'>[]): string; identity(sender: string): { initial: string; color: string }; duration(seconds: number): string }
  var ElonRecordActions: { bind(host: HTMLElement, preview: () => LinkPreview, options: { current: () => boolean; api: (path: string, init: RequestInit) => Promise<unknown> }): () => void }
  var ElonRecordMedia: { mount(host: HTMLElement, row: Pick<RecordRow, 'kind' | 'filename'>, options: { current: () => boolean; scope: string; load: () => Promise<Blob>; openImage: (blob: Blob, name: string, trigger: HTMLElement) => () => void }): () => void }
}
