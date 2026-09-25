import { Grid3X3, LoaderCircle, X } from 'lucide-react'
import { gridCaption, GRID_FIELDS } from './gridChatSnapshot'
import type { GridChatAttachmentController } from './useGridChatAttachment'
import styles from './GridChatAttachmentControl.module.css'

export default function GridChatAttachmentControl({ control }: { control: GridChatAttachmentController }) {
  if (!control.available) return null
  const { attachment, selection, busy, error } = control
  return (
    <div className={styles.host} aria-label="币安网格附件">
      <div className={styles.actions}>
        <button type="button" disabled={busy || !control.canRead} onClick={() => void control.read()}>
          {busy ? <LoaderCircle size={14} className={styles.spinner} /> : <Grid3X3 size={14} />}
          {busy ? '读取网格中…' : attachment ? '重新附带网格' : '附带网格'}
        </button>
        <span className={styles.hint}>{attachment ? '仅随下一条消息发送' : '点击后读取，发送前可预览'}</span>
        {(busy || selection || attachment || error) && (
          <button type="button" className={styles.remove} onClick={control.remove} aria-label="取消或移除网格附件">
            <X size={14} />{attachment ? '移除' : '取消'}
          </button>
        )}
      </div>
      {error && <div className={styles.error} role="alert">
        <span>{error}</span>
        <button type="button" onClick={() => void control.openBinance()} disabled={busy}>打开币安官网</button>
      </div>}
      {selection && <div className={styles.picker}>
        <p>选择要附带的网格（本次读取到 {selection.rows.length} 个）</p>
        {selection.rows.length === 0 && <span>列表为空。可在币安官网检查运行中的网格。</span>}
        {selection.rows.map(row => <button key={row.id} type="button" disabled={busy}
          onClick={() => void control.read(row.id!)}>
          <strong>{gridCaption(row)}</strong>
          <span>{row.lower ?? '未知'} ～ {row.upper ?? '未知'} USDT · {row.status ?? '状态未知'}</span>
        </button>)}
      </div>}
      {attachment && <details className={styles.card}>
        <summary>
          <strong>{gridCaption(attachment.facts)}</strong>
          <span>{new Date(attachment.observedAtMs).toLocaleTimeString()} 读取 · 展开查看发送内容</span>
        </summary>
        <p>仅所选网格；缺失值为“未读取”。有效期 5 分钟，发送不会自动刷新。</p>
        <dl>{Object.entries(GRID_FIELDS).map(([key, label]) => <div key={key}>
          <dt>{label}</dt><dd>{attachment.facts[key as keyof typeof GRID_FIELDS] ?? '未读取'}</dd>
        </div>)}</dl>
      </details>}
    </div>
  )
}
