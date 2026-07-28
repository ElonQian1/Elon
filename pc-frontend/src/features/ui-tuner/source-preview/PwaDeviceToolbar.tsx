import { Camera, Maximize2, RotateCw, Settings2 } from 'lucide-react'
import { useMemo, useState } from 'react'
import {
  PWA_CUSTOM_PRESET_ID,
  PWA_DEVICE_PRESETS,
  pwaDeviceViewportFromPreset,
  rotatePwaDeviceViewport,
  updatePwaDeviceViewportSize,
  type PwaDeviceViewport,
} from './pwaDeviceViewport'
import { capturePwaViewportSnapshot } from './sourcePreviewApi'
import styles from './SourcePreview.module.css'

interface RuntimeViewport {
  width: number
  height: number
  deviceScaleFactor?: number
  visualWidth?: number
  visualHeight?: number
  pointer?: 'coarse' | 'fine' | 'none'
}

interface Props {
  viewport: PwaDeviceViewport
  runtimeViewport?: RuntimeViewport | null
  zoom: number
  projectRoot: string
  sourceRevision: string
  runtimeUrl: string
  route?: { path: string; search: string; hash: string } | null
  onViewportChange: (viewport: PwaDeviceViewport) => void
  onZoom: (zoom: number) => void
  onFit: () => void
}

interface CaptureState {
  phase: 'idle' | 'capturing' | 'captured' | 'failed'
  label: string
  path?: string
}

function rounded(value: number | undefined): string {
  if (!Number.isFinite(value)) return '—'
  return Number(value).toFixed(2).replace(/\.?0+$/, '')
}

export function PwaDeviceToolbar({
  viewport,
  runtimeViewport,
  zoom,
  projectRoot,
  sourceRevision,
  runtimeUrl,
  route,
  onViewportChange,
  onZoom,
  onFit,
}: Props) {
  const [capture, setCapture] = useState<CaptureState>({ phase: 'idle', label: '按当前视口生成 PNG' })
  const zoomOptions = useMemo(() => {
    const values = [.5, .75, 1, 1.25]
    if (!values.some((value) => Math.abs(value - zoom) < .001)) values.push(zoom)
    return values.sort((left, right) => left - right)
  }, [zoom])
  const runtimeMatches = Boolean(
    runtimeViewport
    && Math.abs(runtimeViewport.width - viewport.width) <= 1
    && Math.abs(runtimeViewport.height - viewport.height) <= 1,
  )

  const runCapture = async () => {
    setCapture({ phase: 'capturing', label: '正在生成受控 PNG…' })
    try {
      const result = await capturePwaViewportSnapshot({
        projectRoot,
        sourceRevision,
        runtimeUrl,
        route,
        viewport,
      })
      if (result.ok && result.artifact) {
        setCapture({
          phase: 'captured',
          label: `PNG ${result.artifact.width}×${result.artifact.height} · ${result.artifact.sha256.slice(0, 12)}`,
          path: result.artifact.path,
        })
        return
      }
      setCapture({
        phase: 'failed',
        label: result.diagnostic
          ? `${result.diagnostic.code}：${result.diagnostic.nextStep}`
          : 'PNG 捕获失败，请检查本机节点',
      })
    } catch (error) {
      setCapture({
        phase: 'failed',
        label: error instanceof Error ? error.message : String(error),
      })
    }
  }

  return (
    <section className={styles.pwaDeviceToolbar} aria-label="PWA 设备视口">
      <div className={styles.pwaDeviceToolbarPrimary}>
        <label className={styles.pwaDeviceSelect}>
          <span>设备</span>
          <select
            aria-label="PWA 设备预设"
            value={viewport.presetId}
            onChange={(event) => onViewportChange(pwaDeviceViewportFromPreset(event.target.value, viewport))}
          >
            <option value={PWA_CUSTOM_PRESET_ID}>响应式 / 自定义</option>
            {PWA_DEVICE_PRESETS.map((preset) => (
              <option key={preset.id} value={preset.id}>
                {preset.label} · {preset.width}×{preset.height}
              </option>
            ))}
          </select>
        </label>
        <label className={styles.pwaViewportNumber}>
          <span>宽</span>
          <input
            aria-label="PWA 视口宽度"
            type="number"
            min={240}
            max={1440}
            value={viewport.width}
            onChange={(event) => onViewportChange(updatePwaDeviceViewportSize(viewport, Number(event.target.value), viewport.height))}
          />
        </label>
        <span className={styles.pwaViewportTimes}>×</span>
        <label className={styles.pwaViewportNumber}>
          <span>高</span>
          <input
            aria-label="PWA 视口高度"
            type="number"
            min={240}
            max={2048}
            value={viewport.height}
            onChange={(event) => onViewportChange(updatePwaDeviceViewportSize(viewport, viewport.width, Number(event.target.value)))}
          />
        </label>
        <button
          type="button"
          title="横竖屏旋转"
          aria-label="旋转 PWA 视口"
          onClick={() => onViewportChange(rotatePwaDeviceViewport(viewport))}
        >
          <RotateCw size={14} />
        </button>
        <label className={styles.pwaZoomSelect}>
          <span>缩放</span>
          <select
            aria-label="PWA 画布缩放"
            value={zoom}
            onChange={(event) => onZoom(Number(event.target.value))}
          >
            {zoomOptions.map((value) => <option key={value} value={value}>{Math.round(value * 100)}%</option>)}
          </select>
        </label>
        <button type="button" title="适应当前画布" onClick={onFit}>
          <Maximize2 size={14} />适应
        </button>
        <span className={runtimeMatches ? styles.pwaViewportSynced : styles.pwaViewportPending}>
          {runtimeViewport
            ? `iframe ${runtimeViewport.width}×${runtimeViewport.height}`
            : '等待 iframe 上报'}
        </span>
      </div>
      <details className={styles.pwaDeviceAdvanced}>
        <summary><Settings2 size={13} />高级仿真与证据</summary>
        <div>
          <span>目标 DPR <strong>{rounded(viewport.deviceScaleFactor)}</strong></span>
          <span>浏览器实测 DPR <strong>{rounded(runtimeViewport?.deviceScaleFactor)}</strong></span>
          <span>目标输入 <strong>{viewport.inputMode === 'touch' ? '触控' : '鼠标'}</strong></span>
          <span>浏览器指针 <strong>{runtimeViewport?.pointer || '—'}</strong></span>
          {runtimeViewport?.visualWidth && runtimeViewport.visualHeight
            ? <span>VisualViewport <strong>{rounded(runtimeViewport.visualWidth)}×{rounded(runtimeViewport.visualHeight)}</strong></span>
            : null}
          <label>
            <input
              type="checkbox"
              checked={viewport.showSafeArea}
              onChange={(event) => onViewportChange({ ...viewport, showSafeArea: event.target.checked })}
            />
            显示安全区参考线
          </label>
          <button
            type="button"
            disabled={capture.phase === 'capturing' || !runtimeUrl}
            onClick={() => { void runCapture() }}
          >
            <Camera size={14} />{capture.phase === 'capturing' ? '生成中…' : '捕获当前视口 PNG'}
          </button>
        </div>
        <p data-capture-state={capture.phase} title={capture.path}>{capture.label}</p>
        <small>DPR、触控与安全区在画布中作为目标/参考；PNG 由受控浏览器按目标 DPR 真实渲染。</small>
      </details>
    </section>
  )
}
