import { AlertTriangle, Eye, MonitorSmartphone } from 'lucide-react'
import { useEffect, useState } from 'react'
import { PwaInteractivePreviewSurface } from './PwaInteractivePreviewSurface'
import { SourcePreviewNode } from './SourcePreviewNode'
import type { ComposePreviewRender, SourcePreviewDocument, SourcePreviewFidelity, SourcePreviewMode, SourceRendererCapabilities } from './types'
import type { PwaDesignSession } from './usePwaDesignSession'
import styles from './SourcePreview.module.css'

interface Props {
  document: SourcePreviewDocument | null
  androidRender: ComposePreviewRender | null
  pwaPreview: SourceRendererCapabilities['pwaPreview'] | null
  selectedKey: string | null
  zoom: number
  loading: boolean
  error: string
  onSelect: (key: string) => void
  onModeChange: (mode: SourcePreviewMode) => void
  pwaDesign: PwaDesignSession
}

const UNKNOWN_FIDELITY: SourcePreviewFidelity = {
  score: 0,
  level: 'low',
  safeForDefaultPreview: false,
  totalNodes: 0,
  unsupportedNodes: 0,
  dynamicNodes: 0,
  issues: ['当前 PC 节点尚未提供可还原度评估，为避免误导，暂不默认展示浏览器模拟画面'],
}

export function SourceDrivenPreviewSurface({ document, androidRender, pwaPreview, selectedKey, zoom, loading, error, onSelect, onModeChange, pwaDesign }: Props) {
  const [showAdvancedDraft, setShowAdvancedDraft] = useState(false)
  useEffect(() => setShowAdvancedDraft(false), [document?.sourceRevision])
  const fidelity = document?.fidelity ?? UNKNOWN_FIDELITY
  const usePwaPreview = Boolean(document && !androidRender && pwaPreview?.available && pwaPreview.url)
  const blockUnreliableDraft = Boolean(document && !androidRender && !usePwaPreview && !fidelity.safeForDefaultPreview && !showAdvancedDraft)
  return (
    <div className={styles.surfaceScroller}>
      {loading && <div className={styles.emptyState}>正在解析 Android 源码…</div>}
      {!loading && error && !document && !androidRender && <div className={styles.errorState}>{error}</div>}
      {!loading && !error && !document && !androidRender && <div className={styles.emptyState}>输入本机 Android 项目目录并点击“加载源码”，无需连接手机。</div>}
      {androidRender && <div className={styles.androidPreviewFrame}>
        <div className={styles.rendererTruthBadge}>Android Layoutlib · 真实 Preview</div>
        <img src={androidRender.dataUrl} alt={`${androidRender.composable} Android 真实预览`} />
      </div>}
      {usePwaPreview && document && pwaPreview?.url && (
        <PwaInteractivePreviewSurface url={pwaPreview.url} document={document} zoom={zoom} design={pwaDesign} />
      )}
      {blockUnreliableDraft && document && (
        <section className={styles.fidelityGate} data-testid="source-preview-fidelity-gate">
          <div className={styles.fidelityIcon}><AlertTriangle size={26} /></div>
          <p className={styles.fidelityEyebrow}>源码已经读取 · 已停止展示失真模拟</p>
          <h2>这不是你的真实页面</h2>
          <p className={styles.fidelityLead}>
            当前 XML 包含动态内容或复杂 Android 布局。浏览器无法可靠还原它，为避免新手把残缺草图误认为 APK 效果，系统不会默认显示乱序画面。
          </p>
          <div className={styles.fidelityScoreRow}>
            <strong>本地还原度 {fidelity.score}/100</strong>
            <span>{fidelity.totalNodes ? `${fidelity.totalNodes} 个源码节点` : '等待节点能力升级'}</span>
          </div>
          <ul className={styles.fidelityIssues}>
            {fidelity.issues.slice(0, 4).map((issue) => <li key={issue}>{issue}</li>)}
          </ul>
          <div className={styles.fidelityActions}>
            <button className={styles.fidelityPrimary} type="button" onClick={() => onModeChange('evidence')}>
              <MonitorSmartphone size={17} />查看 Android 真帧
            </button>
            <button type="button" onClick={() => setShowAdvancedDraft(true)}>
              <Eye size={17} />高级：查看结构草图
            </button>
          </div>
          <small>结构草图只用于定位节点和尝试参数，不代表 APK 的字体、位置、圆角或运行时内容。</small>
        </section>
      )}
      {!androidRender && !usePwaPreview && document && !blockUnreliableDraft && (
        <div className={styles.deviceViewport} style={{ width: document.canvas.width * zoom, height: document.canvas.height * zoom }}>
          <div
            className={styles.rendererDraftBadge}
            title={`UI IR ${document.irVersion} · 真相来源：Android 源码`}
          >
            {fidelity.safeForDefaultPreview ? '本地可编辑草稿 · 待 Android 真帧校准' : '高级结构草图 · 不代表 APK 外观'}
          </div>
          <div className={styles.deviceCanvas} style={{ width: document.canvas.width, height: document.canvas.height, background: document.canvas.background, transform: `scale(${zoom})` }} onClick={() => onSelect(document.root.key)}>
            <SourcePreviewNode node={document.root} selectedKey={selectedKey} onSelect={onSelect} />
          </div>
        </div>
      )}
    </div>
  )
}
