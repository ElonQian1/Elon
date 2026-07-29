import { Check, Copy, Pipette } from 'lucide-react'
import { useEffect, useRef, useState, type CSSProperties, type PointerEvent as ReactPointerEvent } from 'react'
import styles from './PwaColorPicker.module.css'

interface Props {
  label: string
  value: string
  placement: 'above' | 'below'
  floatingStyle?: CSSProperties
  onChange: (value: string) => void
}

interface RgbaColor {
  r: number
  g: number
  b: number
  a: number
}

interface HsvColor {
  h: number
  s: number
  v: number
}

type ColorFormat = 'hex' | 'rgba'

const DEFAULT_COLOR: RgbaColor = { r: 0, g: 0, b: 0, a: 1 }
const PRESET_COLORS = [
  '#ef476f', '#ff7f50', '#f59e0b', '#facc15', '#84cc16', '#22c55e',
  '#14b8a6', '#06b6d4', '#0ea5e9', '#3b82f6', '#6366f1', '#8b5cf6',
  '#d946ef', '#ec4899', '#f8fafc', '#cbd5e1', '#64748b', '#111827',
]

export default function PwaColorPopover({ label, value, placement, floatingStyle, onChange }: Props) {
  const initial = parseCssColor(value) ?? DEFAULT_COLOR
  const [color, setColor] = useState(initial)
  const [format, setFormat] = useState<ColorFormat>(() => value.trim().toLowerCase().startsWith('rgb') ? 'rgba' : 'hex')
  const [draft, setDraft] = useState(() => formatColor(initial, format))
  const [copied, setCopied] = useState(false)
  const planeRef = useRef<HTMLDivElement>(null)
  const hsv = rgbToHsv(color)
  const formatted = formatColor(color, format)
  const eyeDropperSupported = typeof window !== 'undefined' && 'EyeDropper' in window

  useEffect(() => {
    const parsed = parseCssColor(value)
    if (parsed) setColor(parsed)
  }, [value])

  useEffect(() => {
    setDraft(formatted)
  }, [formatted])

  const commit = (next: RgbaColor, nextFormat = format) => {
    const normalized = normalizeColor(next)
    setColor(normalized)
    onChange(formatColor(normalized, nextFormat))
  }

  const updateSaturationValue = (event: ReactPointerEvent<HTMLDivElement>) => {
    const rect = planeRef.current?.getBoundingClientRect()
    if (!rect) return
    const saturation = clamp((event.clientX - rect.left) / rect.width, 0, 1)
    const brightness = 1 - clamp((event.clientY - rect.top) / rect.height, 0, 1)
    commit({ ...hsvToRgb({ h: hsv.h, s: saturation, v: brightness }), a: color.a })
  }

  const pickFromScreen = async () => {
    const EyeDropperConstructor = (window as typeof window & {
      EyeDropper?: new () => { open: () => Promise<{ sRGBHex: string }> }
    }).EyeDropper
    if (!EyeDropperConstructor) return
    try {
      const result = await new EyeDropperConstructor().open()
      const picked = parseCssColor(result.sRGBHex)
      if (picked) commit({ ...picked, a: color.a })
    } catch {
      // The browser rejects when the user cancels the picker.
    }
  }

  const copyValue = async () => {
    await navigator.clipboard.writeText(formatted)
    setCopied(true)
    window.setTimeout(() => setCopied(false), 1200)
  }

  const colorStyle = {
    '--picker-hue': hsv.h,
    '--picker-red': color.r,
    '--picker-green': color.g,
    '--picker-blue': color.b,
    '--picker-alpha': color.a,
  } as CSSProperties

  return (
    <div
      className={styles.popover}
      data-placement={placement}
      role="dialog"
      aria-label={`${label}颜色选择器`}
      style={{ ...colorStyle, ...floatingStyle }}
    >
      <div
        ref={planeRef}
        className={styles.saturationPlane}
        data-testid="pwa-color-saturation-value"
        onPointerDown={(event) => {
          event.currentTarget.setPointerCapture(event.pointerId)
          updateSaturationValue(event)
        }}
        onPointerMove={(event) => {
          if (event.currentTarget.hasPointerCapture(event.pointerId)) updateSaturationValue(event)
        }}
      >
        <span
          className={styles.saturationHandle}
          style={{ left: `${hsv.s * 100}%`, top: `${(1 - hsv.v) * 100}%` }}
        />
      </div>

      <div className={styles.controlRow}>
        <button
          type="button"
          className={styles.iconButton}
          aria-label="从屏幕吸取颜色"
          title={eyeDropperSupported ? '从屏幕吸取颜色' : '当前浏览器不支持吸管'}
          disabled={!eyeDropperSupported}
          onClick={() => { void pickFromScreen() }}
        >
          <Pipette size={15} />
        </button>
        <span className={styles.currentColor} style={{ backgroundColor: formatted }} aria-hidden="true" />
        <div className={styles.sliders}>
          <input
            className={styles.hueSlider}
            aria-label="色相"
            type="range"
            min="0"
            max="360"
            value={Math.round(hsv.h)}
            onChange={(event) => {
              const next = hsvToRgb({ ...hsv, h: Number(event.currentTarget.value) })
              commit({ ...next, a: color.a })
            }}
          />
          <input
            className={styles.alphaSlider}
            aria-label="颜色透明度"
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={color.a}
            onChange={(event) => commit({ ...color, a: Number(event.currentTarget.value) })}
          />
        </div>
      </div>

      <div className={styles.valueRow}>
        <select
          aria-label="颜色格式"
          value={format}
          onChange={(event) => {
            const nextFormat = event.currentTarget.value as ColorFormat
            setFormat(nextFormat)
            onChange(formatColor(color, nextFormat))
          }}
        >
          <option value="hex">HEX</option>
          <option value="rgba">RGBA</option>
        </select>
        <input
          aria-label={`${label}精确颜色值`}
          value={draft}
          spellCheck={false}
          onChange={(event) => {
            const nextDraft = event.currentTarget.value
            setDraft(nextDraft)
            const parsed = parseCssColor(nextDraft)
            if (parsed) {
              setColor(parsed)
              onChange(formatColor(parsed, format))
            }
          }}
          onBlur={() => setDraft(formatted)}
        />
        <button type="button" className={styles.iconButton} aria-label="复制颜色值" onClick={() => { void copyValue() }}>
          {copied ? <Check size={15} /> : <Copy size={15} />}
        </button>
      </div>

      <div className={styles.palette} aria-label="常用颜色">
        {PRESET_COLORS.map((preset) => (
          <button
            key={preset}
            type="button"
            aria-label={`使用颜色 ${preset}`}
            title={preset}
            style={{ backgroundColor: preset }}
            onClick={() => {
              const next = parseCssColor(preset)
              if (next) commit(next)
            }}
          />
        ))}
      </div>
    </div>
  )
}

function parseCssColor(input: string): RgbaColor | null {
  const value = input.trim().toLowerCase()
  if (value === 'transparent') return { ...DEFAULT_COLOR, a: 0 }
  const hex = value.match(/^#([0-9a-f]{3,8})$/i)?.[1]
  if (hex) {
    const expanded = hex.length <= 4 ? [...hex].map((digit) => `${digit}${digit}`).join('') : hex
    if (expanded.length === 6 || expanded.length === 8) {
      return {
        r: Number.parseInt(expanded.slice(0, 2), 16),
        g: Number.parseInt(expanded.slice(2, 4), 16),
        b: Number.parseInt(expanded.slice(4, 6), 16),
        a: expanded.length === 8 ? Number.parseInt(expanded.slice(6, 8), 16) / 255 : 1,
      }
    }
  }
  const rgb = value.match(/^rgba?\(\s*([\d.]+)\s*[, ]\s*([\d.]+)\s*[, ]\s*([\d.]+)(?:\s*[,/]\s*([\d.]+)%?)?\s*\)$/)
  if (!rgb) return null
  return normalizeColor({
    r: Number(rgb[1]),
    g: Number(rgb[2]),
    b: Number(rgb[3]),
    a: rgb[4] === undefined ? 1 : Number(rgb[4]) / (value.endsWith('%)') ? 100 : 1),
  })
}

function formatColor(color: RgbaColor, format: ColorFormat): string {
  const normalized = normalizeColor(color)
  if (format === 'rgba') {
    return `rgba(${normalized.r}, ${normalized.g}, ${normalized.b}, ${trimAlpha(normalized.a)})`
  }
  const rgb = [normalized.r, normalized.g, normalized.b]
    .map((channel) => channel.toString(16).padStart(2, '0'))
    .join('')
  const alpha = Math.round(normalized.a * 255).toString(16).padStart(2, '0')
  return `#${rgb}${normalized.a < 1 ? alpha : ''}`
}

function rgbToHsv(color: RgbaColor): HsvColor {
  const red = color.r / 255
  const green = color.g / 255
  const blue = color.b / 255
  const max = Math.max(red, green, blue)
  const min = Math.min(red, green, blue)
  const delta = max - min
  let hue = 0
  if (delta) {
    if (max === red) hue = 60 * (((green - blue) / delta) % 6)
    else if (max === green) hue = 60 * ((blue - red) / delta + 2)
    else hue = 60 * ((red - green) / delta + 4)
  }
  return { h: hue < 0 ? hue + 360 : hue, s: max ? delta / max : 0, v: max }
}

function hsvToRgb(color: HsvColor): Pick<RgbaColor, 'r' | 'g' | 'b'> {
  const chroma = color.v * color.s
  const section = color.h / 60
  const secondary = chroma * (1 - Math.abs((section % 2) - 1))
  const [red, green, blue] = section < 1 ? [chroma, secondary, 0]
    : section < 2 ? [secondary, chroma, 0]
      : section < 3 ? [0, chroma, secondary]
        : section < 4 ? [0, secondary, chroma]
          : section < 5 ? [secondary, 0, chroma]
            : [chroma, 0, secondary]
  const match = color.v - chroma
  return { r: (red + match) * 255, g: (green + match) * 255, b: (blue + match) * 255 }
}

function normalizeColor(color: RgbaColor): RgbaColor {
  return {
    r: Math.round(clamp(color.r, 0, 255)),
    g: Math.round(clamp(color.g, 0, 255)),
    b: Math.round(clamp(color.b, 0, 255)),
    a: clamp(color.a, 0, 1),
  }
}

function trimAlpha(alpha: number): string {
  return alpha.toFixed(2).replace(/0+$/, '').replace(/\.$/, '')
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, Number.isFinite(value) ? value : min))
}
