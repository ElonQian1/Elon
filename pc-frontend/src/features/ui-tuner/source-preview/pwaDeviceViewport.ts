export interface PwaSafeArea {
  top: number
  right: number
  bottom: number
  left: number
}

export interface PwaDevicePreset {
  id: string
  label: string
  detail: string
  width: number
  height: number
  deviceScaleFactor: number
  inputMode: 'touch' | 'mouse'
  safeArea: PwaSafeArea
}

export interface PwaDeviceViewport {
  presetId: string
  width: number
  height: number
  deviceScaleFactor: number
  inputMode: 'touch' | 'mouse'
  safeArea: PwaSafeArea
  showSafeArea: boolean
}

export const PWA_CUSTOM_PRESET_ID = 'responsive'
export const PWA_DEVICE_VIEWPORT_STORAGE_KEY = 'elon.uiTuner.pwaDeviceViewport.v1'

const NO_SAFE_AREA: PwaSafeArea = { top: 0, right: 0, bottom: 0, left: 0 }

export const PWA_DEVICE_PRESETS: readonly PwaDevicePreset[] = [
  {
    id: 'xiaomi-current',
    label: '当前小米真机',
    detail: 'ADB 1080×2400 / 420dpi 的 CSS 逻辑量级',
    width: 412,
    height: 915,
    deviceScaleFactor: 2.625,
    inputMode: 'touch',
    safeArea: { top: 28, right: 0, bottom: 24, left: 0 },
  },
  {
    id: 'android-compact',
    label: 'Android 小屏',
    detail: '常见 360dp 宽度',
    width: 360,
    height: 800,
    deviceScaleFactor: 3,
    inputMode: 'touch',
    safeArea: NO_SAFE_AREA,
  },
  {
    id: 'iphone-x',
    label: 'iPhone X / 11 Pro',
    detail: '刘海与 Home 条安全区',
    width: 375,
    height: 812,
    deviceScaleFactor: 3,
    inputMode: 'touch',
    safeArea: { top: 44, right: 0, bottom: 34, left: 0 },
  },
  {
    id: 'iphone-14',
    label: 'iPhone 13 / 14',
    detail: '现代 iOS 390pt 视口',
    width: 390,
    height: 844,
    deviceScaleFactor: 3,
    inputMode: 'touch',
    safeArea: { top: 47, right: 0, bottom: 34, left: 0 },
  },
  {
    id: 'pixel-7',
    label: 'Pixel 7',
    detail: '现代 Android 412dp 视口',
    width: 412,
    height: 915,
    deviceScaleFactor: 2.625,
    inputMode: 'touch',
    safeArea: NO_SAFE_AREA,
  },
  {
    id: 'small-compat',
    label: '小屏兼容',
    detail: '只用于兼容性，不作为默认手机',
    width: 320,
    height: 640,
    deviceScaleFactor: 2,
    inputMode: 'touch',
    safeArea: NO_SAFE_AREA,
  },
]

function viewportFromPreset(preset: PwaDevicePreset, showSafeArea: boolean): PwaDeviceViewport {
  return {
    presetId: preset.id,
    width: preset.width,
    height: preset.height,
    deviceScaleFactor: preset.deviceScaleFactor,
    inputMode: preset.inputMode,
    safeArea: { ...preset.safeArea },
    showSafeArea,
  }
}

export const DEFAULT_PWA_DEVICE_VIEWPORT: PwaDeviceViewport = viewportFromPreset(PWA_DEVICE_PRESETS[0], false)

function boundedInteger(value: unknown, fallback: number, minimum: number, maximum: number): number {
  const parsed = typeof value === 'number' ? value : Number(value)
  if (!Number.isFinite(parsed)) return fallback
  return Math.max(minimum, Math.min(maximum, Math.round(parsed)))
}

function boundedScaleFactor(value: unknown, fallback: number): number {
  const parsed = typeof value === 'number' ? value : Number(value)
  if (!Number.isFinite(parsed)) return fallback
  return Math.max(.5, Math.min(4, Math.round(parsed * 1000) / 1000))
}

function safeArea(value: unknown, fallback: PwaSafeArea): PwaSafeArea {
  const candidate = value && typeof value === 'object' ? value as Partial<PwaSafeArea> : {}
  return {
    top: boundedInteger(candidate.top, fallback.top, 0, 240),
    right: boundedInteger(candidate.right, fallback.right, 0, 240),
    bottom: boundedInteger(candidate.bottom, fallback.bottom, 0, 240),
    left: boundedInteger(candidate.left, fallback.left, 0, 240),
  }
}

export function normalizePwaDeviceViewport(
  value: Partial<PwaDeviceViewport>,
  fallback: PwaDeviceViewport = DEFAULT_PWA_DEVICE_VIEWPORT,
): PwaDeviceViewport {
  const preset = PWA_DEVICE_PRESETS.find((entry) => entry.id === value.presetId)
  const base = preset ? viewportFromPreset(preset, fallback.showSafeArea) : fallback
  return {
    presetId: preset?.id
      ?? (typeof value.presetId === 'string' ? PWA_CUSTOM_PRESET_ID : base.presetId),
    width: boundedInteger(value.width, base.width, 240, 1440),
    height: boundedInteger(value.height, base.height, 240, 2048),
    deviceScaleFactor: boundedScaleFactor(value.deviceScaleFactor, base.deviceScaleFactor),
    inputMode: value.inputMode === 'mouse' ? 'mouse' : base.inputMode,
    safeArea: safeArea(value.safeArea, base.safeArea),
    showSafeArea: typeof value.showSafeArea === 'boolean' ? value.showSafeArea : base.showSafeArea,
  }
}

export function pwaDeviceViewportFromPreset(
  presetId: string,
  previous: PwaDeviceViewport = DEFAULT_PWA_DEVICE_VIEWPORT,
): PwaDeviceViewport {
  const preset = PWA_DEVICE_PRESETS.find((entry) => entry.id === presetId)
  if (!preset) return { ...previous, presetId: PWA_CUSTOM_PRESET_ID }
  return viewportFromPreset(preset, previous.showSafeArea)
}

export function updatePwaDeviceViewportSize(
  viewport: PwaDeviceViewport,
  width: number,
  height: number,
): PwaDeviceViewport {
  return normalizePwaDeviceViewport({
    ...viewport,
    presetId: PWA_CUSTOM_PRESET_ID,
    width,
    height,
  }, viewport)
}

export function rotatePwaDeviceViewport(viewport: PwaDeviceViewport): PwaDeviceViewport {
  return normalizePwaDeviceViewport({
    ...viewport,
    presetId: PWA_CUSTOM_PRESET_ID,
    width: viewport.height,
    height: viewport.width,
    safeArea: {
      top: viewport.safeArea.left,
      right: viewport.safeArea.top,
      bottom: viewport.safeArea.right,
      left: viewport.safeArea.bottom,
    },
  }, viewport)
}

export function readPwaDeviceViewport(storage?: Pick<Storage, 'getItem'> | null): PwaDeviceViewport {
  const target = storage ?? (typeof window === 'undefined' ? null : window.localStorage)
  if (!target) return DEFAULT_PWA_DEVICE_VIEWPORT
  try {
    const raw = target.getItem(PWA_DEVICE_VIEWPORT_STORAGE_KEY)
    if (!raw) return DEFAULT_PWA_DEVICE_VIEWPORT
    return normalizePwaDeviceViewport(JSON.parse(raw) as Partial<PwaDeviceViewport>)
  } catch {
    return DEFAULT_PWA_DEVICE_VIEWPORT
  }
}

export function savePwaDeviceViewport(
  viewport: PwaDeviceViewport,
  storage?: Pick<Storage, 'setItem'> | null,
): void {
  const target = storage ?? (typeof window === 'undefined' ? null : window.localStorage)
  if (!target) return
  try {
    target.setItem(PWA_DEVICE_VIEWPORT_STORAGE_KEY, JSON.stringify(normalizePwaDeviceViewport(viewport)))
  } catch {
    // 浏览器隐私策略或存储配额不应阻断画布本身。
  }
}
