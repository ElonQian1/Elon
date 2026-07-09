import { useEffect, useState } from 'react'
import { clamp } from './uiTunerGeometry'
import styles from './UiTunerPage.module.css'

function parseDraftNumber(value: string) {
  const trimmed = value.trim()
  if (!trimmed || trimmed === '-' || trimmed === '.' || trimmed === '-.') return null
  const next = Number(trimmed)
  return Number.isFinite(next) ? next : null
}

export function NumberField({
  label,
  value,
  onChange,
  min = -9999,
  max = 9999,
  step = 1,
}: {
  label: string
  value: number
  onChange: (value: number) => void
  min?: number
  max?: number
  step?: number
}) {
  const [draft, setDraft] = useState(String(value))
  const [editing, setEditing] = useState(false)

  useEffect(() => {
    if (!editing) setDraft(String(value))
  }, [editing, value])

  const commitDraft = () => {
    const next = parseDraftNumber(draft)
    if (next === null) {
      setDraft(String(value))
      return
    }
    const clamped = clamp(next, min, max)
    onChange(clamped)
    setDraft(String(clamped))
  }

  return (
    <label className={styles.field}>
      <span>{label}</span>
      <input
        type="number"
        value={draft}
        min={min}
        max={max}
        step={step}
        onBlur={() => {
          setEditing(false)
          commitDraft()
        }}
        onChange={(event) => {
          const raw = event.currentTarget.value
          setEditing(true)
          setDraft(raw)
          const next = parseDraftNumber(raw)
          if (next !== null && next >= min && next <= max) onChange(next)
        }}
        onFocus={() => setEditing(true)}
        onKeyDown={(event) => {
          if (event.key === 'Enter') {
            event.currentTarget.blur()
          } else if (event.key === 'Escape') {
            setDraft(String(value))
            event.currentTarget.blur()
          }
        }}
      />
    </label>
  )
}

export function ColorField({
  label,
  value,
  onChange,
}: {
  label: string
  value: string
  onChange: (value: string) => void
}) {
  return (
    <label className={styles.field}>
      <span>{label}</span>
      <input type="color" value={value} onChange={(event) => onChange(event.currentTarget.value)} />
    </label>
  )
}
