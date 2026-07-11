import { createCalibration } from './comparisonGeometry'
import type { ComparisonCalibration, PixelRect, PixelSize } from './types'
import styles from './UiTunerComparisonWorkspace.module.css'

interface CalibrationPanelProps {
  calibration: ComparisonCalibration
  targetSize: PixelSize
  currentSize: PixelSize
  onChange: (value: ComparisonCalibration) => void
}

export function CalibrationPanel({
  calibration,
  targetSize,
  currentSize,
  onChange,
}: CalibrationPanelProps) {
  const update = (side: 'target' | 'current', edge: keyof PixelRect, value: number) => {
    const target = { ...calibration.targetContentRect }
    const current = { ...calibration.currentContentRect }
    const rect = side === 'target' ? target : current
    rect[edge] = Math.round(value)
    onChange(createCalibration(targetSize, currentSize, target, current))
  }
  return (
    <details className={styles.calibrationPanel}>
      <summary>内容区校准</summary>
      <p>设计稿与真机尺寸不同时，先排除状态栏、导航栏和留白；基础比例由程序计算，不交给 AI 猜。</p>
      <div className={styles.calibrationGrid}>
        <RectFields label="设计稿内容区" rect={calibration.targetContentRect} onChange={(edge, value) => update('target', edge, value)} />
        <RectFields label="真机内容区" rect={calibration.currentContentRect} onChange={(edge, value) => update('current', edge, value)} />
      </div>
      <button type="button" onClick={() => onChange(createCalibration(targetSize, currentSize))}>恢复整屏校准</button>
    </details>
  )
}

function RectFields({
  label,
  rect,
  onChange,
}: {
  label: string
  rect: PixelRect
  onChange: (edge: keyof PixelRect, value: number) => void
}) {
  return (
    <fieldset>
      <legend>{label}</legend>
      {(['left', 'top', 'right', 'bottom'] as const).map((edge) => (
        <label key={edge}>
          <span>{edge}</span>
          <input type="number" value={rect[edge]} onChange={(event) => onChange(edge, Number(event.currentTarget.value))} />
        </label>
      ))}
    </fieldset>
  )
}
