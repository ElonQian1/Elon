import { SourcePreviewNode } from './SourcePreviewNode'
import type { SourcePreviewDocument } from './types'
import styles from './SourcePreview.module.css'

interface Props { document: SourcePreviewDocument | null; selectedKey: string | null; zoom: number; loading: boolean; error: string; onSelect: (key: string) => void }

export function SourceDrivenPreviewSurface({ document, selectedKey, zoom, loading, error, onSelect }: Props) {
  return (
    <div className={styles.surfaceScroller}>
      {loading && <div className={styles.emptyState}>正在解析 Android 源码…</div>}
      {!loading && error && <div className={styles.errorState}>{error}</div>}
      {!loading && !error && !document && <div className={styles.emptyState}>输入本机 Android 项目目录并点击“加载源码”，无需连接手机。</div>}
      {document && (
        <div className={styles.deviceViewport} style={{ width: document.canvas.width * zoom, height: document.canvas.height * zoom }}>
          <div className={styles.deviceCanvas} style={{ width: document.canvas.width, height: document.canvas.height, background: document.canvas.background, transform: `scale(${zoom})` }} onClick={() => onSelect(document.root.key)}>
            <SourcePreviewNode node={document.root} selectedKey={selectedKey} onSelect={onSelect} />
          </div>
        </div>
      )}
    </div>
  )
}
