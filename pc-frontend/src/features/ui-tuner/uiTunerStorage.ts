import type { UiTunerDocument, UiTunerExportElement } from './types'

const STORAGE_KEY = 'elon.pc.uiTuner.document.v2.apk-source'
const STORAGE_KEY_V3 = 'elon.pc.uiTuner.document.v3.runtime-source'
const DEVICE_STORAGE_PREFIX = 'elon.pc.uiTuner.deviceDocument.v1.'

function isUiTunerDocument(value: unknown): value is UiTunerDocument {
  if (!value || typeof value !== 'object') return false
  const candidate = value as Partial<UiTunerDocument>
  return candidate.version === 1
    && Boolean(candidate.canvas)
    && Array.isArray(candidate.elements)
}

export function loadUiTunerDocument(expectedSourceSignature?: string): UiTunerDocument | null {
  if (typeof window === 'undefined') return null
  const rawV3 = window.localStorage.getItem(STORAGE_KEY_V3)
  const raw = rawV3 ?? window.localStorage.getItem(STORAGE_KEY)
  if (!raw) return null
  try {
    const parsed = JSON.parse(raw) as unknown
    if (!isUiTunerDocument(parsed)) return null
    if (rawV3 && (parsed.runtimeSnapshot || parsed.source?.kind === 'device_snapshot')) return parsed
    if (expectedSourceSignature && parsed.source?.signature !== expectedSourceSignature) return null
    return parsed
  } catch {
    return null
  }
}

export function saveUiTunerDocument(document: UiTunerDocument) {
  if (typeof window === 'undefined') return
  window.localStorage.setItem(STORAGE_KEY_V3, JSON.stringify(document))
}

function deviceStorageKey(deviceIdentity: string) {
  return `${DEVICE_STORAGE_PREFIX}${encodeURIComponent(deviceIdentity.trim())}`
}

export function loadUiTunerDeviceDocument(deviceIdentity: string): UiTunerDocument | null {
  if (typeof window === 'undefined' || !deviceIdentity.trim()) return null
  const raw = window.localStorage.getItem(deviceStorageKey(deviceIdentity))
  if (!raw) return null
  try {
    const parsed = JSON.parse(raw) as unknown
    return isUiTunerDocument(parsed) ? parsed : null
  } catch {
    return null
  }
}

export function saveUiTunerDeviceDocument(deviceIdentity: string, document: UiTunerDocument) {
  if (typeof window === 'undefined' || !deviceIdentity.trim()) return
  window.localStorage.setItem(deviceStorageKey(deviceIdentity), JSON.stringify(document))
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
    visibility: element.visibility,
    standard: element.standard,
    runtime: element.runtime,
  }))

  return {
    version: 1,
    source: document.source,
    runtimeSnapshot: document.runtimeSnapshot,
    canvas: document.canvas,
    elements,
    exportedAt: new Date().toISOString(),
  }
}

export function stringifyUiTunerExport(document: UiTunerDocument) {
  return JSON.stringify(buildUiTunerExport(document), null, 2)
}
