import { clamp } from './uiTunerGeometry'
import styles from './UiTunerPage.module.css'

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
  return (
    <label className={styles.field}>
      <span>{label}</span>
      <input
        type="number"
        value={value}
        min={min}
        max={max}
        step={step}
        onChange={(event) => {
          const next = Number(event.currentTarget.value)
          if (Number.isFinite(next)) onChange(clamp(next, min, max))
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
