import { lazy, Suspense, useEffect, useLayoutEffect, useRef, useState, type CSSProperties } from 'react'
import { createPortal } from 'react-dom'
import type { PwaStyleProperty } from './pwaDesignDraft'
import type { PwaDesignSession } from './usePwaDesignSession'
import pickerStyles from './PwaColorPicker.module.css'
import styles from './PwaStyleInspector.module.css'

const PwaColorPopover = lazy(() => import('./PwaColorPopover'))
const POPOVER_WIDTH = 284
const POPOVER_HEIGHT = 330
const POPOVER_GAP = 7
const VIEWPORT_GUTTER = 10

interface Props {
  label: string
  property: PwaStyleProperty
  value: string
  session: PwaDesignSession
}

export default function PwaColorStyleFieldControl({ label, property, value, session }: Props) {
  const [open, setOpen] = useState(false)
  const [placement, setPlacement] = useState<'above' | 'below'>('below')
  const [floatingStyle, setFloatingStyle] = useState<CSSProperties>({
    position: 'fixed',
    top: 0,
    left: 0,
    right: 'auto',
    bottom: 'auto',
    width: POPOVER_WIDTH,
    maxHeight: 'calc(100vh - 20px)',
    overflow: 'auto',
  })
  const shellRef = useRef<HTMLDivElement>(null)
  const popoverRef = useRef<HTMLDivElement>(null)

  useLayoutEffect(() => {
    if (!open) return
    const updatePosition = () => {
      const rect = shellRef.current?.getBoundingClientRect()
      if (!rect) return
      const width = Math.min(POPOVER_WIDTH, window.innerWidth - VIEWPORT_GUTTER * 2)
      const spaceBelow = window.innerHeight - rect.bottom
      const nextPlacement = spaceBelow < POPOVER_HEIGHT + POPOVER_GAP && rect.top > spaceBelow ? 'above' : 'below'
      const preferredTop = nextPlacement === 'above'
        ? rect.top - POPOVER_HEIGHT - POPOVER_GAP
        : rect.bottom + POPOVER_GAP
      setPlacement(nextPlacement)
      setFloatingStyle({
        position: 'fixed',
        top: Math.max(VIEWPORT_GUTTER, Math.min(preferredTop, window.innerHeight - POPOVER_HEIGHT - VIEWPORT_GUTTER)),
        left: Math.max(VIEWPORT_GUTTER, Math.min(rect.right - width, window.innerWidth - width - VIEWPORT_GUTTER)),
        right: 'auto',
        bottom: 'auto',
        width,
        maxHeight: 'calc(100vh - 20px)',
        overflow: 'auto',
      })
    }
    updatePosition()
    window.addEventListener('resize', updatePosition)
    document.addEventListener('scroll', updatePosition, true)
    return () => {
      window.removeEventListener('resize', updatePosition)
      document.removeEventListener('scroll', updatePosition, true)
    }
  }, [open])

  useEffect(() => {
    if (!open) return
    const closeOnOutsidePress = (event: PointerEvent) => {
      const target = event.target as Node
      if (!shellRef.current?.contains(target) && !popoverRef.current?.contains(target)) setOpen(false)
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
    <div ref={shellRef} className={`${styles.styleField} ${pickerStyles.fieldShell}`}>
      <span className={styles.fieldLabel}><span>{label}</span><small>CSS</small></span>
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
          onFocus={(event) => event.currentTarget.select()}
          onChange={(event) => session.updateStyle(property, event.currentTarget.value)}
        />
      </div>
      {open && createPortal(
        <div ref={popoverRef}>
          <Suspense fallback={<div className={pickerStyles.popover} style={floatingStyle}>正在打开颜色面板…</div>}>
            <PwaColorPopover
              label={label}
              value={value}
              placement={placement}
              floatingStyle={floatingStyle}
              onChange={(next) => session.updateStyle(property, next)}
            />
          </Suspense>
        </div>,
        document.body,
      )}
    </div>
  )
}
