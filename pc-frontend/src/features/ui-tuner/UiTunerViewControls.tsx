import { Filter, RotateCcw, Search } from 'lucide-react'
import type { UiTunerFilterResult, UiTunerFilterState, UiTunerViewMode } from './filtering'
import styles from './UiTunerPanels.module.css'

interface UiTunerViewControlsProps {
  filter: UiTunerFilterState
  result: UiTunerFilterResult
  onChange: (patch: Partial<UiTunerFilterState>) => void
  onReset: () => void
}

const VIEW_MODES: Array<{ id: UiTunerViewMode; label: string }> = [
  { id: 'product', label: '产品' },
  { id: 'layout', label: '布局' },
  { id: 'source', label: '源码' },
  { id: 'debug', label: '全部' },
]

export function UiTunerViewControls({
  filter,
  result,
  onChange,
  onReset,
}: UiTunerViewControlsProps) {
  return (
    <div className={styles.layerFilters}>
      <div className={styles.filterHeader}>
        <Filter size={14} aria-hidden="true" />
        <strong>{result.visible.length}/{result.totalCount}</strong>
        <span>隐藏 {result.hiddenCount}</span>
        <button type="button" onClick={onReset} aria-label="重置图层过滤" title="重置图层过滤">
          <RotateCcw size={13} aria-hidden="true" />
        </button>
      </div>

      <div className={styles.modeSegments} aria-label="图层显示模式">
        {VIEW_MODES.map((mode) => (
          <button
            key={mode.id}
            type="button"
            className={filter.mode === mode.id ? styles.activeSegment : ''}
            onClick={() => onChange({ mode: mode.id })}
          >
            {mode.label}
          </button>
        ))}
      </div>

      <label className={styles.searchField}>
        <Search size={13} aria-hidden="true" />
        <input
          value={filter.query}
          onChange={(event) => onChange({ query: event.currentTarget.value })}
          placeholder="搜索 id / 文本 / 源码"
        />
      </label>

      <div className={styles.filterToggles}>
        <FilterToggle
          label="目标包"
          checked={filter.onlyTargetPackage}
          onChange={(checked) => onChange({ onlyTargetPackage: checked })}
        />
        <FilterToggle
          label="源码"
          checked={filter.onlySourceMapped}
          onChange={(checked) => onChange({ onlySourceMapped: checked })}
        />
        <FilterToggle
          label="可点"
          checked={filter.onlyInteractive}
          onChange={(checked) => onChange({ onlyInteractive: checked })}
        />
        <FilterToggle
          label="结构"
          checked={filter.showStructural}
          onChange={(checked) => onChange({ showStructural: checked })}
        />
        <FilterToggle
          label="隐藏"
          checked={filter.showHidden}
          onChange={(checked) => onChange({ showHidden: checked })}
        />
      </div>

      <label className={styles.filterRange}>
        <span>最小边</span>
        <input
          type="range"
          min={0}
          max={48}
          step={2}
          value={filter.minSize}
          onChange={(event) => onChange({ minSize: Number(event.currentTarget.value) })}
        />
        <strong>{filter.minSize}px</strong>
      </label>

      <div className={styles.filterStats}>
        <span>源码 {result.sourceMappedCount}</span>
        <span>结构 {result.structuralCount}</span>
        <span>重复 {result.duplicateCount}</span>
        <span>同组件 {result.repeatedInstanceCount}</span>
      </div>
    </div>
  )
}

interface FilterToggleProps {
  label: string
  checked: boolean
  onChange: (checked: boolean) => void
}

function FilterToggle({ label, checked, onChange }: FilterToggleProps) {
  return (
    <label className={styles.filterToggle}>
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.currentTarget.checked)}
      />
      <span>{label}</span>
    </label>
  )
}
