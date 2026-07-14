import { Columns2, Eye, PanelLeftClose, PanelLeftOpen, ScanSearch, Sparkles, Trash2 } from 'lucide-react'
import type { ComparisonMode, TargetCurrentPair } from './types'
import styles from './UiTunerComparisonWorkspace.module.css'

const MODES: Array<{ id: ComparisonMode; label: string; icon: typeof Columns2 }> = [
  { id: 'split', label: '左右', icon: Columns2 },
  { id: 'overlay', label: '叠加', icon: Eye },
  { id: 'blink', label: '闪烁', icon: Sparkles },
  { id: 'diff', label: '差异', icon: ScanSearch },
]

interface ComparisonModeControlsProps {
  mode: ComparisonMode
  opacity: number
  pair: TargetCurrentPair | null
  targetReady: boolean
  designPaneOpen: boolean
  onModeChange: (mode: ComparisonMode) => void
  onToggleDesignPane: () => void
  onOpacityChange: (opacity: number) => void
  onClearPair: () => void
}

export function ComparisonModeControls({
  mode,
  opacity,
  pair,
  targetReady,
  designPaneOpen,
  onModeChange,
  onToggleDesignPane,
  onOpacityChange,
  onClearPair,
}: ComparisonModeControlsProps) {
  return (
    <div className={styles.comparisonBar}>
      <div className={styles.modeSegments} aria-label="设计稿对比模式">
        {MODES.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            type="button"
            className={mode === id ? styles.activeMode : ''}
            disabled={!targetReady && id !== 'split'}
            onClick={() => onModeChange(id)}
          >
            <Icon size={13} aria-hidden="true" />
            {label}
          </button>
        ))}
        {targetReady && mode === 'split' && (
          <button type="button" onClick={onToggleDesignPane}>
            {designPaneOpen
              ? <PanelLeftClose size={13} aria-hidden="true" />
              : <PanelLeftOpen size={13} aria-hidden="true" />}
            {designPaneOpen ? '隐藏设计稿' : '显示设计稿'}
          </button>
        )}
      </div>
      {mode === 'overlay' && (
        <label className={styles.opacityControl}>
          <span>设计稿 {Math.round(opacity * 100)}%</span>
          <input
            type="range"
            min={0.05}
            max={0.95}
            step={0.05}
            value={opacity}
            onChange={(event) => onOpacityChange(Number(event.currentTarget.value))}
          />
        </label>
      )}
      <div className={styles.pairStatus}>
        {pair ? (
          <>
            <span title={pair.definitionId}>已配对 · {pair.definitionId}</span>
            <button type="button" onClick={onClearPair} title="清除目标区域和节点配对">
              <Trash2 size={13} aria-hidden="true" />
            </button>
          </>
        ) : (
          <span>{targetReady ? '左侧框选，再点击右侧节点' : '请先导入设计稿'}</span>
        )}
      </div>
    </div>
  )
}
