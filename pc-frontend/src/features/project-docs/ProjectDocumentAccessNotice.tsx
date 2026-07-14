import { AlertTriangle, RefreshCw, WifiOff } from 'lucide-react'

import type { DocumentCatalog } from './projectDocumentModel'
import styles from './ProjectDocumentsWorkspace.module.css'

interface Props {
  access?: DocumentCatalog['access']
  warnings?: string[]
  error?: string
  path?: string
  onRetry: () => void
}

export default function ProjectDocumentAccessNotice({ access, warnings = [], error, path, onRetry }: Props) {
  if (error) {
    return (
      <div className={styles.documentFailure} role="alert">
        <WifiOff size={30} aria-hidden="true" />
        <strong>这篇文档的正文暂时无法读取</strong>
        {path && <code>{path}</code>}
        <span>{error}</span>
        <p>目录仍可显示不代表绑定的 PC 节点能够读取正文。恢复节点后可直接重试，不需要重新整理文档。</p>
        <button type="button" onClick={onRetry}><RefreshCw size={14} aria-hidden="true" />重新读取正文</button>
      </div>
    )
  }

  if (!access?.degraded && warnings.length === 0) return null
  return (
    <div className={styles.accessNotice} role="status">
      <AlertTriangle size={16} aria-hidden="true" />
      <div>
        <strong>{access?.degraded ? '当前正在浏览服务器回退副本' : '项目文档有读取提示'}</strong>
        <span>{warnings[0] || '绑定的 PC 节点暂不可用；为避免写入旧副本，编辑和整理操作已经暂停。'}</span>
      </div>
      <button type="button" onClick={onRetry}><RefreshCw size={13} aria-hidden="true" />重试节点</button>
    </div>
  )
}
