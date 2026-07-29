import type { PwaStyleProperty } from './pwaDesignDraft'
import type { PwaDesignSession, PwaSelection } from './usePwaDesignSession'
import styles from './SourcePreview.module.css'

export interface PwaStyleFieldSpec {
  property: PwaStyleProperty
  label: string
  placeholder?: string
  quickStep?: number
  quickUnit?: string
  min?: number
  max?: number
}

export const SIZE_FIELDS: PwaStyleFieldSpec[] = [
  { property: 'width', label: '宽度', placeholder: 'auto / 100% / 320px', quickStep: 4, quickUnit: 'px', min: 1 },
  { property: 'height', label: '高度', placeholder: 'auto / 100% / 48px', quickStep: 4, quickUnit: 'px', min: 1 },
]

export const PADDING_FIELDS: PwaStyleFieldSpec[] = [
  { property: 'paddingTop', label: '上', quickStep: 2, quickUnit: 'px', min: 0 },
  { property: 'paddingRight', label: '右', quickStep: 2, quickUnit: 'px', min: 0 },
  { property: 'paddingBottom', label: '下', quickStep: 2, quickUnit: 'px', min: 0 },
  { property: 'paddingLeft', label: '左', quickStep: 2, quickUnit: 'px', min: 0 },
]

export const MARGIN_FIELDS: PwaStyleFieldSpec[] = [
  { property: 'marginTop', label: '上', quickStep: 2, quickUnit: 'px' },
  { property: 'marginRight', label: '右', quickStep: 2, quickUnit: 'px' },
  { property: 'marginBottom', label: '下', quickStep: 2, quickUnit: 'px' },
  { property: 'marginLeft', label: '左', quickStep: 2, quickUnit: 'px' },
]

export const TYPE_FIELDS: PwaStyleFieldSpec[] = [
  { property: 'fontSize', label: '字号', placeholder: '16px / 1rem', quickStep: 1, quickUnit: 'px', min: 8 },
  { property: 'fontWeight', label: '字重', placeholder: '400 / 700', quickStep: 100, min: 100, max: 900 },
  { property: 'lineHeight', label: '行高', placeholder: '1.5 / 24px', quickStep: 1, quickUnit: 'px', min: 8 },
  { property: 'borderRadius', label: '圆角', placeholder: '8px / 50%', quickStep: 2, quickUnit: 'px', min: 0 },
]

export function originalValue(selection: PwaSelection, property: PwaStyleProperty): string {
  return selection.originalStyle.authored[property]
    || selection.originalStyle.computed[property]
    || ''
}

export function fieldValue(session: PwaDesignSession, property: PwaStyleProperty): string {
  const selection = session.selection
  if (!selection) return ''
  return Object.values(session.draft?.elements ?? {}).find((element) => (
    element.identity.key === selection.identity.key || element.identity.selector === selection.identity.selector
  ))?.styleDiff[property]
    ?? originalValue(selection, property)
}

function normalizeNumber(value: number, min?: number, max?: number): number {
  const clampedMin = typeof min === 'number' ? Math.max(min, value) : value
  return typeof max === 'number' ? Math.min(max, clampedMin) : clampedMin
}

function formatNumber(value: number): string {
  return Number.isInteger(value) ? String(value) : value.toFixed(2).replace(/\.?0+$/, '')
}

export function adjustedCssValue(current: string, delta: number, fallbackUnit = '', min?: number, max?: number): string {
  const text = current.trim()
  const match = text.match(/^(-?\d+(?:\.\d+)?)([a-z%]*)$/i)
  if (match) {
    const unit = match[2] || fallbackUnit
    return `${formatNumber(normalizeNumber(Number(match[1]) + delta, min, max))}${unit}`
  }
  const fallback = fallbackUnit || 'px'
  return `${formatNumber(normalizeNumber(delta > 0 ? delta : 0, min, max))}${fallback}`
}

export function StyleField({ session, spec }: { session: PwaDesignSession; spec: PwaStyleFieldSpec }) {
  const value = fieldValue(session, spec.property)
  const canQuickAdjust = typeof spec.quickStep === 'number'
  const quickAdjust = (direction: -1 | 1) => {
    if (!canQuickAdjust) return
    session.updateStyle(
      spec.property,
      adjustedCssValue(value, spec.quickStep! * direction, spec.quickUnit, spec.min, spec.max),
    )
  }
  return (
    <label className={styles.pwaStyleField}>
      <span>{spec.label}</span>
      <div className={styles.pwaQuickAdjust}>
        {canQuickAdjust && <button type="button" onClick={() => quickAdjust(-1)} aria-label={`${spec.label}减小`}>−</button>}
        <input
          value={value}
          placeholder={spec.placeholder}
          onChange={(event) => session.updateStyle(spec.property, event.currentTarget.value)}
        />
        {canQuickAdjust && <button type="button" onClick={() => quickAdjust(1)} aria-label={`${spec.label}增大`}>+</button>}
      </div>
    </label>
  )
}

export function EdgeFields({ session, fields }: { session: PwaDesignSession; fields: PwaStyleFieldSpec[] }) {
  return <div className={styles.pwaEdgeGrid}>{fields.map((spec) => <StyleField key={spec.property} session={session} spec={spec} />)}</div>
}
