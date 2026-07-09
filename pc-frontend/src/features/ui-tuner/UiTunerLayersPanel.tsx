import { Plus, Type } from 'lucide-react'
import type { UiTunerElement, UiTunerElementKind } from './types'
import { kindLabel } from './uiTunerGeometry'
import styles from './UiTunerPage.module.css'

interface UiTunerLayersPanelProps {
  elements: UiTunerElement[]
  selectedId: string | null
  onAddElement: (kind: UiTunerElementKind) => void
  onSelectElement: (id: string) => void
}

export function UiTunerLayersPanel({
  elements,
  selectedId,
  onAddElement,
  onSelectElement,
}: UiTunerLayersPanelProps) {
  return (
    <aside className={styles.layersPanel}>
      <div className={styles.panelHeader}>
        <h1>微调画布</h1>
        <p>导入 APP 截图作为真实底图，再拖动图层调位置、尺寸、字号和间距。</p>
      </div>

      <div className={styles.addGroup}>
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
      </div>

      <div className={styles.layerList} aria-label="画布图层">
        {elements.map((element) => (
          <button
            key={element.id}
            type="button"
            className={[styles.layerItem, element.id === selectedId ? styles.activeLayer : ''].join(' ')}
            onClick={() => onSelectElement(element.id)}
          >
            <span>{kindLabel(element.kind)}</span>
            <strong>{element.name}</strong>
            <small>{element.width} x {element.height}</small>
          </button>
        ))}
      </div>
    </aside>
  )
}
