import type { PwaStyleProperty } from './pwaDesignDraft'
import type { PwaDesignSession } from './usePwaDesignSession'
import styles from './SourcePreview.module.css'

interface Props {
  label: string
  property: PwaStyleProperty
  value: string
  session: PwaDesignSession
}

export function PwaColorStyleField({ label, property, value, session }: Props) {
  const pickerValue = cssColorToHex(value) ?? '#000000'
  return (
    <label className={styles.pwaStyleField}>
      <span>{label}</span>
      <div className={styles.pwaColorField}>
        <input
          aria-label={`${label}取色器`}
          type="color"
          value={pickerValue}
          onChange={(event) => session.updateStyle(property, event.currentTarget.value)}
        />
        <input
          aria-label={`${label}颜色值`}
          value={value}
          placeholder="#222255 / rgba(34,34,85,.9)"
          onChange={(event) => session.updateStyle(property, event.currentTarget.value)}
        />
      </div>
    </label>
  )
}

function cssColorToHex(value: string): string | null {
  const text = value.trim()
  const hex = text.match(/^#([0-9a-f]{6})$/i)
  if (hex) return `#${hex[1].toLowerCase()}`
  const short = text.match(/^#([0-9a-f])([0-9a-f])([0-9a-f])$/i)
  if (short) return `#${short[1]}${short[1]}${short[2]}${short[2]}${short[3]}${short[3]}`.toLowerCase()
  const rgb = text.match(/^rgba?\((\d+),\s*(\d+),\s*(\d+)/i)
  if (!rgb) return null
  return `#${[rgb[1], rgb[2], rgb[3]].map((part) => Number(part).toString(16).padStart(2, '0')).join('')}`
}
