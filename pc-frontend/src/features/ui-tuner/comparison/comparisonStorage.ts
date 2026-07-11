import type { UiTunerDocument, UiTunerReferenceImage } from '../types'
import type { ComparisonWorkspaceState } from './types'

const STORAGE_PREFIX = 'elon.uiTuner.comparison.v1'

function smallHash(value: string) {
  let hash = 2166136261
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index)
    hash = Math.imul(hash, 16777619)
  }
  return (hash >>> 0).toString(16).padStart(8, '0')
}

export function targetDesignId(image?: UiTunerReferenceImage) {
  if (!image) return 'no-target'
  const sample = `${image.name}:${image.width}x${image.height}:${image.dataUrl.length}:${image.dataUrl.slice(-192)}`
  return `target-${smallHash(sample)}`
}

export function comparisonStorageKey(document: UiTunerDocument) {
  const project = document.runtimeSnapshot?.sourceFingerprint
    ?? document.runtimeSnapshot?.packageName
    ?? document.source?.signature
    ?? 'local'
  return `${STORAGE_PREFIX}:${smallHash(project)}:${targetDesignId(document.canvas.targetDesign)}`
}

export function loadComparisonState(key: string, designId: string): ComparisonWorkspaceState | null {
  if (typeof window === 'undefined') return null
  const raw = window.localStorage.getItem(key)
  if (!raw) return null
  try {
    const parsed = JSON.parse(raw) as Partial<ComparisonWorkspaceState>
    if (parsed.version !== 1 || parsed.targetDesignId !== designId) return null
    return {
      version: 1,
      targetDesignId: designId,
      mode: parsed.mode ?? 'split',
      overlayOpacity: typeof parsed.overlayOpacity === 'number' ? parsed.overlayOpacity : 0.5,
      targetRect: parsed.targetRect ?? null,
      pair: parsed.pair?.projectedTargetRect && parsed.pair.calibrationId ? parsed.pair : null,
      calibration: parsed.calibration?.targetSize && parsed.calibration.currentSize
        ? parsed.calibration
        : null,
    }
  } catch {
    return null
  }
}

export function saveComparisonState(key: string, state: ComparisonWorkspaceState) {
  if (typeof window === 'undefined') return
  window.localStorage.setItem(key, JSON.stringify(state))
}
