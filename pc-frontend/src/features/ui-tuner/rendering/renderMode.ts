import type { SourcePreviewMode } from '../source-preview/types'

export interface UiTunerRenderModeInput {
  workspaceMode: SourcePreviewMode
  hasAndroidPixels: boolean
  runtimeDocument: boolean
}

export interface UiTunerRenderMode {
  /** The canvas is showing pixels produced by Android, even when it is read-only. */
  androidVisual: boolean
  /** The current document is instrumented and can accept deterministic LIVE patches. */
  runtimeEditable: boolean
}

export function deriveUiTunerRenderMode({
  workspaceMode,
  hasAndroidPixels,
  runtimeDocument,
}: UiTunerRenderModeInput): UiTunerRenderMode {
  const androidVisual = workspaceMode === 'evidence' && hasAndroidPixels
  return {
    androidVisual,
    runtimeEditable: androidVisual && runtimeDocument,
  }
}
