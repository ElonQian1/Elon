import { Move, Trash2 } from 'lucide-react'
import { DEFAULT_UI_TUNER_FILTER } from './filtering'
import type { UiTunerFilterResult } from './filtering'
import type { UiTunerStandardInsight } from './standards'
import type { UiTunerDocument, UiTunerElement } from './types'
import { UiTunerCodexPanel } from './UiTunerCodexPanel'
import { ColorField, NumberField } from './UiTunerFields'
import { UiTunerStandardsPanel } from './UiTunerStandardsPanel'
import type { MetricItem } from './uiTunerGeometry'
import type { UiTunerCodexContextPack } from './contextPack'
import type { UiTunerVerificationReport } from './runtime/verification'
import { UiTunerLivePanel } from './live/UiTunerLivePanel'
import type { useLiveUiSession } from './live/useLiveUiSession'
import styles from './UiTunerPage.module.css'

const MIN_SIZE = 24
const DEFAULT_CANVAS_MAX = 10000

interface UiTunerInspectorProps {
  tunerDoc: UiTunerDocument
  selected: UiTunerElement | null
  metrics: MetricItem[]
  filterResult: UiTunerFilterResult
  standardInsight: UiTunerStandardInsight | null
  exportJson: string
  onUpdateCanvas: (patch: Partial<UiTunerDocument['canvas']>) => void
  onUpdateElement: (id: string, patch: Partial<UiTunerElement>) => void
  onDeleteSelected: () => void
  onCopyCliPatch: () => void
  onCopyStandardPackage: () => void
  onApplyStandard: (standard: UiTunerElement['standard']) => void
  onSetProductMode: () => void
  onSetDebugMode: () => void
  verificationReport: UiTunerVerificationReport | null
  onMutationTaskStarted: (pack: UiTunerCodexContextPack) => Promise<void> | void
  onRequestVerification: () => void
  liveUi: ReturnType<typeof useLiveUiSession>
  onLiveOptimisticUpdate: (patch: Partial<UiTunerElement>) => void
}

export function UiTunerInspector({
  tunerDoc,
  selected,
  metrics,
  filterResult,
  standardInsight,
  exportJson,
  onUpdateCanvas,
  onUpdateElement,
  onDeleteSelected,
  onCopyCliPatch,
  onCopyStandardPackage,
  onApplyStandard,
  onSetProductMode,
  onSetDebugMode,
  verificationReport,
  onMutationTaskStarted,
  onRequestVerification,
  liveUi,
  onLiveOptimisticUpdate,
}: UiTunerInspectorProps) {
  return (
    <aside className={styles.inspector}>
      <CanvasSection
        tunerDoc={tunerDoc}
        filterResult={filterResult}
        onUpdateCanvas={onUpdateCanvas}
        onSetProductMode={onSetProductMode}
        onSetDebugMode={onSetDebugMode}
      />

      {selected ? (
        <>
          <SelectedSection
            tunerDoc={tunerDoc}
            selected={selected}
            metrics={metrics}
            onUpdateElement={onUpdateElement}
            onDeleteSelected={onDeleteSelected}
          />
          <UiTunerLivePanel
            state={liveUi.state}
            error={liveUi.error}
            busy={liveUi.busy}
            session={liveUi.session}
            node={liveUi.selectedNode}
            onApply={liveUi.apply}
            onUndo={liveUi.undo}
            onRedo={liveUi.redo}
            onReconnect={liveUi.reconnect}
            onOptimisticUpdate={onLiveOptimisticUpdate}
          />
          <UiTunerStandardsPanel
            insight={standardInsight}
            appliedStandard={selected.standard}
            onApplyStandard={onApplyStandard}
            onCopyStandardPackage={onCopyStandardPackage}
          />
          <UiTunerCodexPanel
            tunerDoc={tunerDoc}
            selected={selected}
            metrics={metrics}
            filterResult={filterResult}
            standardInsight={standardInsight}
            verificationReport={verificationReport}
            onMutationTaskStarted={onMutationTaskStarted}
            onRequestVerification={onRequestVerification}
          />
          <GeometrySection tunerDoc={tunerDoc} selected={selected} onUpdateElement={onUpdateElement} />
          <TypographySection selected={selected} onUpdateElement={onUpdateElement} />
          <AppearanceSection selected={selected} onUpdateElement={onUpdateElement} />
        </>
      ) : (
        <section className={styles.emptyState}>
          <Move size={18} aria-hidden="true" />
          <p>点击画布上的板块后，可在这里调位置、字号、行高、内边距和颜色。</p>
        </section>
      )}

      <section className={styles.section}>
        <h2>导出参数</h2>
        <textarea className={styles.exportBox} value={exportJson} readOnly />
      </section>

      <div className={styles.inspectorFooterActions}>
        <button type="button" onClick={onCopyCliPatch}>复制 CLI 包</button>
        <button type="button" onClick={onCopyStandardPackage}>复制标准草案</button>
      </div>
    </aside>
  )
}

interface CanvasSectionProps {
  tunerDoc: UiTunerDocument
  filterResult: UiTunerFilterResult
  onUpdateCanvas: (patch: Partial<UiTunerDocument['canvas']>) => void
  onSetProductMode: () => void
  onSetDebugMode: () => void
}

function CanvasSection({
  tunerDoc,
  filterResult,
  onUpdateCanvas,
  onSetProductMode,
  onSetDebugMode,
}: CanvasSectionProps) {
  return (
    <section className={styles.section}>
      <h2>画布</h2>
      <label className={styles.fieldFull}>
        <span>名称</span>
        <input value={tunerDoc.canvas.name} onChange={(event) => onUpdateCanvas({ name: event.currentTarget.value })} />
      </label>
      <div className={styles.gridFields}>
        <NumberField label="宽" value={tunerDoc.canvas.width} min={280} max={DEFAULT_CANVAS_MAX} onChange={(width) => onUpdateCanvas({ width })} />
        <NumberField label="高" value={tunerDoc.canvas.height} min={360} max={DEFAULT_CANVAS_MAX} onChange={(height) => onUpdateCanvas({ height })} />
      </div>
      <ColorField label="背景" value={tunerDoc.canvas.background} onChange={(background) => onUpdateCanvas({ background })} />
      {tunerDoc.canvas.referenceImage && (
        <ReferenceImagePanel tunerDoc={tunerDoc} onUpdateCanvas={onUpdateCanvas} />
      )}
      {tunerDoc.runtimeSnapshot && (
        <>
          <div className={styles.sourcePanel}>
            <span>真机 XML 快照</span>
            <strong>{tunerDoc.runtimeSnapshot.packageName ?? 'APK'} · {tunerDoc.runtimeSnapshot.deviceId}</strong>
            <small>
              {tunerDoc.runtimeSnapshot.nodeCount} nodes
              {`\n当前显示: ${filterResult.visible.length}/${filterResult.totalCount}`}
              {`\n已过滤: 结构 ${filterResult.structuralCount} · 重复 ${filterResult.duplicateCount}`}
              {tunerDoc.runtimeSnapshot.activityName ? `\n${tunerDoc.runtimeSnapshot.activityName}` : ''}
              {tunerDoc.runtimeSnapshot.sourceRoot ? `\n${tunerDoc.runtimeSnapshot.sourceRoot}` : ''}
            </small>
          </div>
          <div className={styles.inlineActions}>
            <button type="button" onClick={onSetProductMode}>产品模式</button>
            <button type="button" onClick={onSetDebugMode}>全部 XML</button>
          </div>
        </>
      )}
      {tunerDoc.source && (
        <div className={styles.sourcePanel}>
          <span>来源</span>
          <strong>{tunerDoc.source.label}</strong>
          <small>{tunerDoc.source.files?.join('\n') ?? tunerDoc.source.signature}</small>
        </div>
      )}
    </section>
  )
}

interface ReferenceImagePanelProps {
  tunerDoc: UiTunerDocument
  onUpdateCanvas: (patch: Partial<UiTunerDocument['canvas']>) => void
}

function ReferenceImagePanel({ tunerDoc, onUpdateCanvas }: ReferenceImagePanelProps) {
  const referenceImage = tunerDoc.canvas.referenceImage!
  return (
    <>
      <div className={styles.sourcePanel}>
        <span>APP 截图底图</span>
        <strong>{referenceImage.name}</strong>
        <small>{referenceImage.width} x {referenceImage.height}</small>
      </div>
      <label className={styles.rangeField}>
        <span>底图透明</span>
        <input
          type="range"
          min={0.15}
          max={1}
          step={0.05}
          value={referenceImage.opacity}
          onChange={(event) => onUpdateCanvas({
            referenceImage: { ...referenceImage, opacity: Number(event.currentTarget.value) },
          })}
        />
        <strong>{Math.round(referenceImage.opacity * 100)}%</strong>
      </label>
      <div className={styles.inlineActions}>
        <button
          type="button"
          onClick={() => onUpdateCanvas({ referenceImage: { ...referenceImage, visible: !referenceImage.visible } })}
        >
          {referenceImage.visible ? '隐藏截图' : '显示截图'}
        </button>
        <button type="button" onClick={() => onUpdateCanvas({ referenceImage: undefined })}>
          移除截图
        </button>
      </div>
    </>
  )
}

interface SelectedSectionProps {
  tunerDoc: UiTunerDocument
  selected: UiTunerElement
  metrics: MetricItem[]
  onUpdateElement: (id: string, patch: Partial<UiTunerElement>) => void
  onDeleteSelected: () => void
}

function SelectedSection({
  selected,
  metrics,
  onUpdateElement,
  onDeleteSelected,
}: SelectedSectionProps) {
  return (
    <section className={styles.section}>
      <div className={styles.sectionHeader}>
        <h2>{selected.name}</h2>
        <button type="button" onClick={onDeleteSelected} aria-label="删除选中元素">
          <Trash2 size={14} aria-hidden="true" />
        </button>
      </div>
      <label className={styles.fieldFull}>
        <span>图层名</span>
        <input value={selected.name} onChange={(event) => onUpdateElement(selected.id, { name: event.currentTarget.value })} />
      </label>
      <label className={styles.fieldFull}>
        <span>文本</span>
        <textarea value={selected.text} onChange={(event) => onUpdateElement(selected.id, { text: event.currentTarget.value })} />
      </label>
      <div className={styles.metricGrid}>
        {metrics.map((metric) => (
          <div key={metric.label}>
            <span>{metric.label}</span>
            <strong>{metric.value}</strong>
          </div>
        ))}
      </div>
      {selected.source && (
        <div className={styles.sourcePanel}>
          <span>源码来源</span>
          <strong>{selected.source.token ?? selected.source.label}</strong>
          <small>
            {selected.source.file}
            {selected.source.line ? `:${selected.source.line}` : ''}
            {selected.source.rawValue ? `\n${selected.source.rawValue}` : ''}
          </small>
        </div>
      )}
      {selected.runtime && (
        <div className={styles.sourcePanel}>
          <span>运行时节点</span>
          <strong>{selected.runtime.resourceId ?? selected.runtime.className ?? selected.runtime.nodeId}</strong>
          <small>
            {selected.runtime.xpath}
            {`\n原始 bounds: ${selected.runtime.originalBounds.left},${selected.runtime.originalBounds.top} ${selected.runtime.originalBounds.width}x${selected.runtime.originalBounds.height}`}
          </small>
        </div>
      )}
    </section>
  )
}

interface GeometrySectionProps {
  tunerDoc: UiTunerDocument
  selected: UiTunerElement
  onUpdateElement: (id: string, patch: Partial<UiTunerElement>) => void
}

interface ElementSectionProps {
  selected: UiTunerElement
  onUpdateElement: (id: string, patch: Partial<UiTunerElement>) => void
}

function GeometrySection({ tunerDoc, selected, onUpdateElement }: GeometrySectionProps) {
  return (
    <section className={styles.section}>
      <h2>位置和尺寸</h2>
      <div className={styles.gridFields}>
        <NumberField label="X" value={selected.x} min={0} max={tunerDoc.canvas.width} onChange={(x) => onUpdateElement(selected.id, { x })} />
        <NumberField label="Y" value={selected.y} min={0} max={tunerDoc.canvas.height} onChange={(y) => onUpdateElement(selected.id, { y })} />
        <NumberField label="W" value={selected.width} min={MIN_SIZE} max={tunerDoc.canvas.width} onChange={(width) => onUpdateElement(selected.id, { width })} />
        <NumberField label="H" value={selected.height} min={MIN_SIZE} max={tunerDoc.canvas.height} onChange={(height) => onUpdateElement(selected.id, { height })} />
      </div>
    </section>
  )
}

function TypographySection({ selected, onUpdateElement }: ElementSectionProps) {
  return (
    <section className={styles.section}>
      <h2>文字和间距</h2>
      <div className={styles.gridFields}>
        <NumberField label="字号" value={selected.fontSize} min={8} max={96} onChange={(fontSize) => onUpdateElement(selected.id, { fontSize })} />
        <NumberField label="行高" value={selected.lineHeight} min={8} max={120} onChange={(lineHeight) => onUpdateElement(selected.id, { lineHeight })} />
        <NumberField label="字距" value={selected.letterSpacing} min={-2} max={12} onChange={(letterSpacing) => onUpdateElement(selected.id, { letterSpacing })} />
        <NumberField label="内距 X" value={selected.paddingX} min={0} max={80} onChange={(paddingX) => onUpdateElement(selected.id, { paddingX })} />
        <NumberField label="内距 Y" value={selected.paddingY} min={0} max={80} onChange={(paddingY) => onUpdateElement(selected.id, { paddingY })} />
        <NumberField label="圆角" value={selected.borderRadius} min={0} max={48} onChange={(borderRadius) => onUpdateElement(selected.id, { borderRadius })} />
      </div>
      <label className={styles.fieldFull}>
        <span>字重</span>
        <select value={selected.fontWeight} onChange={(event) => onUpdateElement(selected.id, { fontWeight: Number(event.currentTarget.value) })}>
          <option value={400}>400</option>
          <option value={500}>500</option>
          <option value={600}>600</option>
          <option value={700}>700</option>
          <option value={800}>800</option>
        </select>
      </label>
    </section>
  )
}

function AppearanceSection({ selected, onUpdateElement }: ElementSectionProps) {
  return (
    <section className={styles.section}>
      <h2>外观</h2>
      <div className={styles.gridFields}>
        <ColorField label="文字" value={selected.color} onChange={(color) => onUpdateElement(selected.id, { color })} />
        <ColorField label="背景" value={selected.background} onChange={(background) => onUpdateElement(selected.id, { background })} />
        <ColorField label="边框" value={selected.borderColor} onChange={(borderColor) => onUpdateElement(selected.id, { borderColor })} />
        <NumberField label="边框" value={selected.borderWidth} min={0} max={8} onChange={(borderWidth) => onUpdateElement(selected.id, { borderWidth })} />
      </div>
      <label className={styles.rangeField}>
        <span>透明度</span>
        <input
          type="range"
          min={0.2}
          max={1}
          step={0.05}
          value={selected.opacity}
          onChange={(event) => onUpdateElement(selected.id, { opacity: Number(event.currentTarget.value) })}
        />
        <strong>{Math.round(selected.opacity * 100)}%</strong>
      </label>
    </section>
  )
}

export function buildDebugFilter() {
  return {
    ...DEFAULT_UI_TUNER_FILTER,
    mode: 'debug' as const,
    minSize: 0,
    showHidden: true,
    showStructural: true,
    onlyTargetPackage: false,
  }
}
