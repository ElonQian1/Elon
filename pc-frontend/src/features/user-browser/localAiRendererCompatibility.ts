import type { LocalAiStructuredContentPart } from './localAiBrowserProtocol'
import {
  isYilongRichContent,
  YILONG_RICH_CONTENT_SCHEMA,
} from './richContentProtocol'

export type LocalAiRendererCompatibilityReason =
  | 'unsupported_schema'
  | 'unsupported_kind'
  | 'invalid_payload'
  | 'unsupported_rich_part'
  | 'incomplete_extraction'

export interface LocalAiRendererCompatibilityNotice {
  reason: LocalAiRendererCompatibilityReason
}

const KNOWN_RICH_KINDS = new Set(['finance', 'weather', 'media_gallery', 'map'])
const NATIVE_FALLBACK_GAPS = new Set([
  'image',
  'audio',
  'video',
  'artifact',
  'chart',
  'map',
  'interactive',
])

export function localAiRendererCompatibility(
  parts: LocalAiStructuredContentPart[],
  extractionIncomplete = false,
): LocalAiRendererCompatibilityNotice | undefined {
  if (extractionIncomplete) return { reason: 'incomplete_extraction' }
  for (const part of parts) {
    if (part.type === 'rich_card') {
      if (isYilongRichContent(part.richContent)) continue
      const raw = (part as { richContent?: unknown }).richContent
      if (!isRecord(raw) || raw.schema !== YILONG_RICH_CONTENT_SCHEMA) {
        return { reason: 'unsupported_schema' }
      }
      if (typeof raw.kind !== 'string' || !KNOWN_RICH_KINDS.has(raw.kind)) {
        return { reason: 'unsupported_kind' }
      }
      return { reason: 'invalid_payload' }
    }
    if (NATIVE_FALLBACK_GAPS.has(part.type)) {
      return { reason: 'unsupported_rich_part' }
    }
  }
  return undefined
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value))
}
