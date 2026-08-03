import { Code2, MonitorSmartphone, PanelsTopLeft } from 'lucide-react'
import type { SourcePreviewMode } from '../source-preview/types'
import styles from './UiWorkspaceModeBar.module.css'

interface Props {
  mode: SourcePreviewMode
  onModeChange: (mode: SourcePreviewMode) => void
  status?: string
  compact?: boolean
}

export function UiWorkspaceModeBar({ mode, onModeChange, status, compact = false }: Props) {
  return (
    <div className={`${styles.bar} ${compact ? styles.compact : ''}`} data-testid="ui-workspace-mode-bar">
      <div className={styles.tabs} aria-label="画布渲染方式">
        <button
          type="button"
          className={mode === 'headless' ? styles.active : ''}
          onClick={() => onModeChange('headless')}
        >
          <PanelsTopLeft size={15} />
          多端后台
        </button>
        <button
          type="button"
          className={mode === 'evidence' ? styles.active : ''}
          onClick={() => onModeChange('evidence')}
        >
          <MonitorSmartphone size={15} />
          Android 真帧
        </button>
        <button
          type="button"
          className={mode === 'source' ? styles.active : ''}
          onClick={() => onModeChange('source')}
        >
          <Code2 size={15} />
          本地草稿
        </button>
      </div>
      {!compact && <>
        <span className={styles.status}>
          {status ?? (mode === 'headless'
            ? 'AI 通过 designSession 在后台读取页面、UI 树与证据哈希'
            : mode === 'evidence'
              ? '真实 Android 是最终权威；选区与草稿保持同步'
              : '数字孪生即时编辑；写回前由 Android 真帧校准')}
        </span>
        <span className={styles.compareHint}>导入设计稿后可使用左右、叠加、闪烁和差异模式</span>
      </>}
    </div>
  )
}

