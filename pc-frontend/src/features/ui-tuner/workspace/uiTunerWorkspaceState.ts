import type { UiTunerDocument } from '../types'

export interface UiTunerHistoryState {
  past: UiTunerDocument[]
  future: UiTunerDocument[]
}

export const UI_TUNER_MIN_SIZE = 24
export const UI_TUNER_HISTORY_LIMIT = 80
export const DEFAULT_ANDROID_PACKAGE = 'com.elon.app'
export const WORKSPACE_MODE_STORAGE_KEY = 'elon.uiTuner.workspaceMode.v2'
