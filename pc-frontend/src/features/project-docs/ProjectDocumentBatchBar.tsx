import { CheckSquare2, Pin, RotateCcw, Sparkles, Star, X } from 'lucide-react'

import type { DocumentNavigationMode } from './projectDocumentArchitecture'
import type { DocumentSection } from './projectDocumentSections'
import styles from './ProjectDocumentsWorkspace.module.css'

interface Props {
  count: number
  navigationMode: DocumentNavigationMode
  sections: DocumentSection[]
  busy: boolean
  onAssign: (sectionKey: string) => void
  onPin: () => void
  onRecommend: () => void
  onRestoreAutomatic: () => void
  onAskAi: () => void
  onClear: () => void
}

export default function ProjectDocumentBatchBar({
  count, navigationMode, sections, busy, onAssign, onPin, onRecommend, onRestoreAutomatic, onAskAi, onClear,
}: Props) {
  if (!count) return null
  return (
    <div className={styles.batchBar} aria-label={`已选择 ${count} 份文档`}>
      <strong><CheckSquare2 size={14} />{count} 份</strong>
      <select
        aria-label={navigationMode === 'knowledge' ? '批量移动到知识主题' : '批量调整治理属性'}
        value=""
        disabled={busy}
        onChange={(event) => { if (event.target.value) onAssign(event.target.value) }}
      >
        <option value="">{navigationMode === 'knowledge' ? '移动到主题…' : '治理状态…'}</option>
        {sections.map((section) => <option value={section.key} key={section.key}>{section.label}</option>)}
      </select>
      <button type="button" title="固定到顶部" disabled={busy} onClick={onPin}><Pin size={13} /></button>
      <button type="button" title="加入推荐阅读" disabled={busy} onClick={onRecommend}><Star size={13} /></button>
      <button type="button" title="恢复自动分类" disabled={busy} onClick={onRestoreAutomatic}><RotateCcw size={13} /></button>
      <button type="button" title="让 AI 整理所选文档" disabled={busy} onClick={onAskAi}><Sparkles size={13} /></button>
      <button type="button" title="清除选择" onClick={onClear}><X size={13} /></button>
    </div>
  )
}
