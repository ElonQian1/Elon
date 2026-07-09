import type { UiTunerDocument, UiTunerElement, UiTunerElementKind } from './types'
import { APK_STYLE_SOURCE_SIGNATURE, createApkStyleDocument } from './apkStyleDocument'

function element(
  id: string,
  name: string,
  kind: UiTunerElementKind,
  overrides: Partial<UiTunerElement>,
): UiTunerElement {
  return {
    id,
    name,
    kind,
    x: 24,
    y: 24,
    width: 160,
    height: 48,
    text: name,
    fontSize: 14,
    lineHeight: 20,
    fontWeight: 600,
    letterSpacing: 0,
    paddingX: 12,
    paddingY: 10,
    borderRadius: 6,
    borderWidth: 1,
    color: '#f5f5f5',
    background: '#1f2023',
    borderColor: '#34363b',
    opacity: 1,
    ...overrides,
  }
}

export function createInitialTunerDocument(): UiTunerDocument {
  return createApkStyleDocument()
}

export { APK_STYLE_SOURCE_SIGNATURE }

export function createBlankElement(kind: UiTunerElementKind, index: number): UiTunerElement {
  const baseName = kind === 'text' ? '文字' : kind === 'button' ? '按钮' : kind === 'media' ? '图片占位' : '卡片'
  return element(`custom.${kind}.${Date.now()}`, `${baseName} ${index}`, kind, {
    x: 36 + (index % 3) * 16,
    y: 36 + (index % 5) * 20,
    width: kind === 'text' ? 220 : 180,
    height: kind === 'text' ? 48 : kind === 'button' ? 42 : 112,
    text: kind === 'media' ? '图片/截图' : `${baseName} ${index}`,
    fontSize: kind === 'button' ? 14 : 16,
    lineHeight: kind === 'button' ? 20 : 24,
    fontWeight: kind === 'text' ? 700 : 600,
    paddingX: kind === 'text' ? 0 : 14,
    paddingY: kind === 'text' ? 0 : 12,
    borderWidth: kind === 'text' ? 0 : 1,
    color: kind === 'button' ? '#000000' : '#f5f5f5',
    background: kind === 'button' ? '#f5f5f5' : kind === 'text' ? '#111317' : '#202124',
    borderColor: kind === 'button' ? '#f5f5f5' : '#34363b',
  })
}
