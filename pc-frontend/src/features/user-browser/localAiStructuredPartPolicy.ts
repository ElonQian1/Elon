import type { LocalAiStructuredContentPart } from './localAiBrowserProtocol'

const EMPTY_IMAGE_LABEL = /^(?:图片|图像|image|photo|picture)+$/i

export function shouldRenderNativeStructuredPart(part: LocalAiStructuredContentPart) {
  if (part.type !== 'image') return true
  const label = part.text
    .normalize('NFKC')
    .replace(/[\s\u200b-\u200d\ufeff]+/g, '')
  return Boolean(label && !EMPTY_IMAGE_LABEL.test(label))
}
