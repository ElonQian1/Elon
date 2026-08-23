import type { LocalAiStructuredContentPart } from './localAiBrowserProtocol'

export function shouldRenderNativeStructuredPart(part: LocalAiStructuredContentPart) {
  // The native fallback currently has no media renderer. Rendering extracted image
  // metadata as a card therefore produces a row of fake empty "图片" tiles. The
  // live official surface owns actual images; fallback text and citations remain.
  if (part.type === 'image') return false
  if (part.type === 'rich_card') return Boolean(part.richContent)
  return true
}
