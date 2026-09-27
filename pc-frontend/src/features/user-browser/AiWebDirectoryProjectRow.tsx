import { useState } from 'react'
import { ChevronDown, ChevronRight, FolderClosed, MessageSquare, SquarePen } from 'lucide-react'
import type { AiWebChatBackend } from './useAiWebChatBackend'
import { localAiDirectoryModel, type LocalAiDirectoryRow } from './localAiDirectoryModel'
import styles from './AiWebChatSidebar.module.css'

export default function AiWebDirectoryProjectRow({ item, web }: { item: LocalAiDirectoryRow; web: AiWebChatBackend }) {
  const [expanded, setExpanded] = useState(false)
  const [status, setStatus] = useState('')
  const children = localAiDirectoryModel(web.controller.navigationSnapshot).children(item.id)
  async function toggle() {
    setExpanded(!expanded)
    if (expanded) return
    await sync()
  }
  async function sync() {
    if (web.controller.busyAction) { setStatus('稍后重试同步'); return }
    setStatus('同步中…')
    const result = await web.controller.run('list_conversations', item.id)
    setStatus(result?.commandResult?.ok ? '' : '未完成同步，可重试')
  }
  return <div>
    <div className={styles.projectRow}>
      <button className={styles.directoryItem} type="button" aria-expanded={expanded} data-active={item.active} title={item.title} onClick={() => void toggle()}>
        <FolderClosed size={16} /><span>{item.title}</span>{expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
      </button>
      {expanded && <button className={styles.projectAction} type="button" title="打开项目" aria-label="打开项目"
        onClick={() => void web.controller.run('open_project', item.path)}><SquarePen size={16} /></button>}
    </div>
    {expanded && <div className={styles.projectChildren}>
      {children.map(child => <button className={styles.directoryItem} type="button" key={child.id} title={child.title} data-active={child.active}
        onClick={() => void web.controller.run('open_conversation', child.path)}><MessageSquare size={14} /><span>{child.title}</span></button>)}
      {status ? <button className={styles.directoryItem} type="button" disabled={Boolean(web.controller.busyAction)} onClick={() => void sync()}><span>{status}</span></button>
        : !children.length && <p className={styles.empty}>暂无会话</p>}
    </div>}
  </div>
}
