import { SourcePreviewNode } from './SourcePreviewNode'
import type { ComposePreviewRender, SourcePreviewDocument } from './types'
import styles from './SourcePreview.module.css'

interface Props { document: SourcePreviewDocument | null; androidRender: ComposePreviewRender | null; selectedKey: string | null; zoom: number; loading: boolean; error: string; onSelect: (key: string) => void }

export function SourceDrivenPreviewSurface({ document, androidRender, selectedKey, zoom, loading, error, onSelect }: Props) {
  return (
    <div className={styles.surfaceScroller}>
      {loading && <div className={styles.emptyState}>正在解析 Android 源码…</div>}
      {!loading && error && !document && !androidRender && <div className={styles.errorState}>{error}</div>}
      {!loading && !error && !document && !androidRender && <div className={styles.emptyState}>输入本机 Android 项目目录并点击“加载源码”，无需连接手机。</div>}
      {androidRender && <div className={styles.androidPreviewFrame}>
        <div className={styles.rendererTruthBadge}>Android Layoutlib · 真实 Preview</div>
        <img src={androidRender.dataUrl} alt={`${androidRender.composable} Android 真实预览`} />
      </div>}
      {!androidRender && document && (
        <div className={styles.deviceViewport} style={{ width: document.canvas.width * zoom, height: document.canvas.height * zoom }}>
          <div className={styles.rendererDraftBadge}>React 数字孪生 · 本地草稿，待 Android 校准</div>
          <div className={styles.deviceCanvas} style={{ width: document.canvas.width, height: document.canvas.height, background: document.canvas.background, transform: `scale(${zoom})` }} onClick={() => onSelect(document.root.key)}>
            <SourcePreviewNode node={document.root} selectedKey={selectedKey} onSelect={onSelect} />
          </div>
        </div>
      )}
    </div>
  )
}
