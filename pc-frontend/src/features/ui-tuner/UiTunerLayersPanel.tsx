import { Eye, EyeOff, Lock, PanelLeft, Plus, Type, Unlock } from 'lucide-react'
import type { UiTunerFilterResult, UiTunerFilterState } from './filtering'
import type { UiTunerElementKind } from './types'
import { kindLabel } from './uiTunerGeometry'
import { UiTunerViewControls } from './UiTunerViewControls'
import panelStyles from './UiTunerPanels.module.css'
import styles from './UiTunerPage.module.css'

interface UiTunerLayersPanelProps {
  realRenderer?: boolean
  filter: UiTunerFilterState
  filterResult: UiTunerFilterResult
  selectedId: string | null
  onAddElement: (kind: UiTunerElementKind) => void
  onApplyAppSidebarTemplate: () => void
  onFilterChange: (patch: Partial<UiTunerFilterState>) => void
  onResetFilter: () => void
  onSelectElement: (id: string) => void
  onToggleElementVisibility: (id: string) => void
  onToggleElementLock: (id: string) => void
}

export function UiTunerLayersPanel({
  realRenderer = false,
  filter,
  filterResult,
  selectedId,
  onAddElement,
  onApplyAppSidebarTemplate,
  onFilterChange,
  onResetFilter,
  onSelectElement,
  onToggleElementVisibility,
  onToggleElementLock,
}: UiTunerLayersPanelProps) {
  return (
    <aside className={styles.layersPanel}>
      <div className={styles.panelHeader}>
        <h1>{realRenderer ? 'Android 真实组件' : '微调画布'}</h1>
        <p>{realRenderer
          ? '这里列出 APK 当前画面的真实 Runtime 节点；点选后修改会直接作用于 Android 组件。'
          : '导入 APP 截图作为真实底图，再拖动图层调位置、尺寸、字号和间距。'}</p>
      </div>

      {!realRenderer && <div className={styles.templateGroup}>
        <button type="button" onClick={onApplyAppSidebarTemplate}>
          <PanelLeft size={14} aria-hidden="true" />
          APP 侧边栏模板
        </button>
      </div>}

      {!realRenderer && <div className={styles.addGroup}>
        <button type="button" onClick={() => onAddElement('text')}>
          <Type size={14} aria-hidden="true" />
          文字
        </button>
        <button type="button" onClick={() => onAddElement('card')}>
          <Plus size={14} aria-hidden="true" />
          卡片
        </button>
        <button type="button" onClick={() => onAddElement('button')}>
          <Plus size={14} aria-hidden="true" />
          按钮
        </button>
      </div>}

      <UiTunerViewControls
        filter={filter}
        result={filterResult}
        onChange={onFilterChange}
        onReset={onResetFilter}
      />

      <div className={styles.layerList} aria-label="画布图层">
        {filterResult.groups.map((group) => (
          <section key={group.key} className={panelStyles.layerGroup}>
            <h2>{group.label}<span>{group.items.length}</span></h2>
            {group.items.map(({ element, analysis }) => (
              <div
                key={element.id}
                className={[
                  panelStyles.layerItem,
                  element.id === selectedId ? panelStyles.activeLayer : '',
                  analysis.appearance !== 'solid' ? panelStyles.mutedLayer : '',
                ].join(' ')}
              >
                <button
                  type="button"
                  className={panelStyles.layerIconButton}
                  onClick={() => onToggleElementVisibility(element.id)}
                  aria-label={element.visibility === 'hidden' ? '显示图层' : '隐藏图层'}
                  title={element.visibility === 'hidden' ? '显示图层' : '隐藏图层'}
                >
                  {element.visibility === 'hidden'
                    ? <EyeOff size={13} aria-hidden="true" />
                    : <Eye size={13} aria-hidden="true" />}
                </button>
                <button
                  type="button"
                  className={panelStyles.layerSelectButton}
                  onClick={() => onSelectElement(element.id)}
                >
                  <span>{kindLabel(element.kind)}</span>
                  <strong>
                    {element.name}
                    {analysis.repeatCount > 1 ? ` × ${analysis.repeatCount}` : ''}
                  </strong>
                  <small>
                    {element.width} x {element.height}
                    {analysis.hiddenReasons.length ? ` · ${analysis.hiddenReasons[0]}` : ''}
                  </small>
                </button>
                <button
                  type="button"
                  className={panelStyles.layerIconButton}
                  onClick={() => onToggleElementLock(element.id)}
                  aria-label={element.visibility === 'locked' ? '解锁图层' : '锁定图层'}
                  title={element.visibility === 'locked' ? '解锁图层' : '锁定图层'}
                >
                  {element.visibility === 'locked'
                    ? <Lock size={13} aria-hidden="true" />
                    : <Unlock size={13} aria-hidden="true" />}
                </button>
              </div>
            ))}
          </section>
        ))}
        {filterResult.visible.length === 0 && (
          <div className={panelStyles.layerEmpty}>
            没有符合当前过滤条件的图层
          </div>
        )}
      </div>
    </aside>
  )
}
