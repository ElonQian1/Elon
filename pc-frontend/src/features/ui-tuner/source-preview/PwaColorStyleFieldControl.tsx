import { lazy, Suspense, useEffect, useLayoutEffect, useRef, useState } from 'react'
import type { PwaStyleProperty } from './pwaDesignDraft'
import type { PwaDesignSession } from './usePwaDesignSession'
import pickerStyles from './PwaColorPicker.module.css'
import styles from './SourcePreview.module.css'

const PwaColorPopover = lazy(() => import('./PwaColorPopover'))

interface Props {
  label: string
  property: PwaStyleProperty
  value: string
  session: PwaDesignSession
}

export default function PwaColorStyleFieldControl({ label, property, value, session }: Props) {
  const [open, setOpen] = useState(false)
  const [placement, setPlacement] = useState<'above' | 'below'>('below')
  const shellRef = useRef<HTMLDivElement>(null)

  useLayoutEffect(() => {
    if (!open) return
    const rect = shellRef.current?.getBoundingClientRect()
    if (!rect) return
    const spaceBelow = window.innerHeight - rect.bottom
    setPlacement(spaceBelow < 360 && rect.top > spaceBelow ? 'above' : 'below')
  }, [open])

  useEffect(() => {
    if (!open) return
    const closeOnOutsidePress = (event: PointerEvent) => {
      if (!shellRef.current?.contains(event.target as Node)) setOpen(false)
    }
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpen(false)
    }
    document.addEventListener('pointerdown', closeOnOutsidePress)
    document.addEventListener('keydown', closeOnEscape)
    return () => {
      document.removeEventListener('pointerdown', closeOnOutsidePress)
      document.removeEventListener('keydown', closeOnEscape)
    }
  }, [open])

  return (
    <div ref={shellRef} className={`${styles.pwaStyleField} ${pickerStyles.fieldShell}`}>
      <span>{label}</span>
      <div className={pickerStyles.fieldControl}>
        <button
          type="button"
          className={pickerStyles.swatchButton}
          aria-label={`打开${label}颜色选择器`}
          aria-haspopup="dialog"
          aria-expanded={open}
          onClick={() => setOpen((current) => !current)}
        >
          <span className={pickerStyles.swatch} style={{ backgroundColor: value }} />
        </button>
        <input
          aria-label={`${label}颜色值`}
          value={value}
          placeholder="#222255 / rgba(34,34,85,.9)"
          onChange={(event) => session.updateStyle(property, event.currentTarget.value)}
        />
      </div>
      {open && (
        <Suspense fallback={<div className={pickerStyles.popover} data-placement={placement}>正在打开颜色面板…</div>}>
          <PwaColorPopover
            label={label}
            value={value}
            placement={placement}
            onChange={(next) => session.updateStyle(property, next)}
          />
        </Suspense>
      )}
    </div>
  )
}
