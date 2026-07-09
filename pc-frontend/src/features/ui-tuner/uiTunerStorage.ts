import type { UiTunerDocument, UiTunerExportElement } from './types'

const STORAGE_KEY = 'elon.pc.uiTuner.document.v2.apk-source'

function isUiTunerDocument(value: unknown): value is UiTunerDocument {
  if (!value || typeof value !== 'object') return false
  const candidate = value as Partial<UiTunerDocument>
  return candidate.version === 1
    && Boolean(candidate.canvas)
    && Array.isArray(candidate.elements)
}

export function loadUiTunerDocument(expectedSourceSignature?: string): UiTunerDocument | null {
  if (typeof window === 'undefined') return null
  const raw = window.localStorage.getItem(STORAGE_KEY)
  if (!raw) return null
  try {
    const parsed = JSON.parse(raw) as unknown
    if (!isUiTunerDocument(parsed)) return null
    if (expectedSourceSignature && parsed.source?.signature !== expectedSourceSignature) return null
    return parsed
  } catch {
    return null
  }
}

export function saveUiTunerDocument(document: UiTunerDocument) {
  if (typeof window === 'undefined') return
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(document))
}

export function buildUiTunerExport(document: UiTunerDocument) {
  const elements: UiTunerExportElement[] = document.elements.map((element) => ({
    id: element.id,
    name: element.name,
    kind: element.kind,
    rect: {
      x: element.x,
      y: element.y,
      width: element.width,
      height: element.height,
    },
    text: element.text,
    style: {
      fontSize: element.fontSize,
      lineHeight: element.lineHeight,
      fontWeight: element.fontWeight,
      letterSpacing: element.letterSpacing,
      paddingX: element.paddingX,
      paddingY: element.paddingY,
      borderRadius: element.borderRadius,
      borderWidth: element.borderWidth,
      color: element.color,
      background: element.background,
      borderColor: element.borderColor,
      opacity: element.opacity,
    },
    source: element.source,
  }))

  return {
    version: 1,
    source: document.source,
    canvas: document.canvas,
    elements,
    exportedAt: new Date().toISOString(),
  }
}

export function stringifyUiTunerExport(document: UiTunerDocument) {
  return JSON.stringify(buildUiTunerExport(document), null, 2)
}
